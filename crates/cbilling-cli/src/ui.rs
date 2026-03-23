use ratatui::{
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Padding, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, Wrap,
    },
    Frame,
};

use crate::app::{App, InputMode, ProviderDataState, SortColumn};

// Terminal-safe: all text uses Reset (inherits terminal fg/bg).
// Only use named ANSI colors for semantic highlights that must stand out.
// Dim = Reset + DIM modifier (works on both light/dark backgrounds).
const C_ACCENT: Color = Color::Cyan; // interactive elements, active items
const C_TITLE: Color = Color::Cyan; // section titles, headers
const C_COST_HIGH: Color = Color::Red; // cost > 10,000
const C_COST_MID: Color = Color::Yellow; // cost > 1,000
const C_COST_LOW: Color = Color::Green; // cost < 1,000
const C_BORDER: Color = Color::Reset; // borders follow terminal default
const C_HIGHLIGHT_BG: Color = Color::DarkGray; // selected row background
const C_TOTAL: Color = Color::Green; // total cost number
const C_ERR: Color = Color::Red; // error messages
const C_UP: Color = Color::Red; // cost increased
const C_DOWN: Color = Color::Green; // cost decreased
const C_SEARCH_BG: Color = Color::Reset;
const C_TEXT: Color = Color::Reset; // primary text — terminal default

pub fn draw(frame: &mut Frame, app: &mut App) {
    let has_search = app.input_mode == InputMode::Search || !app.search_query.is_empty();

    // Summary height: 2 (total + blank) + providers + loading + errors + 2 (border)
    let provider_count = app.cost_summary().len()
        + app.loading_errors().len()
        + app
            .data
            .values()
            .filter(|s| matches!(s, ProviderDataState::Loading))
            .count();
    let content_lines = 2 + provider_count; // total line + blank + providers
    let summary_h = (content_lines as u16 + 2) // +2 for border
        .max(5) // minimum useful height
        .min(frame.area().height / 2) // never more than half screen
        .min(18); // hard cap

    let mut constraints = vec![
        Constraint::Length(3),         // tabs
        Constraint::Length(summary_h), // summary (dynamic)
    ];
    if has_search {
        constraints.push(Constraint::Length(3));
    }
    constraints.push(Constraint::Min(6)); // table
    constraints.push(Constraint::Length(1)); // footer

    let outer = Layout::vertical(constraints).split(frame.area());

    let mut idx = 0;
    render_tabs(frame, app, outer[idx]);
    idx += 1;
    render_summary(frame, app, outer[idx]);
    idx += 1;
    if has_search {
        render_search_bar(frame, app, outer[idx]);
        idx += 1;
    }
    render_table(frame, app, outer[idx]);
    idx += 1;
    render_footer(frame, app, outer[idx]);

    if app.show_help {
        render_help(frame);
    }
}

// ── Tabs ────────────────────────────────────────────────────────────────

fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let tab_labels: Vec<(String, bool)> = app
        .providers
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let badge = if name == "All" {
                let totals = app.totals_by_currency();
                if totals.is_empty() {
                    String::new()
                } else {
                    let parts: Vec<String> = totals
                        .iter()
                        .map(|(c, v)| format!("{}{}", format_cost(*v), c))
                        .collect();
                    format!(" [{}]", parts.join("+"))
                }
            } else {
                match app.data.get(name) {
                    Some(ProviderDataState::Loading) => format!(" {}", app.spinner_char()),
                    Some(ProviderDataState::Loaded(d)) => {
                        format!(" {} {}", format_cost(d.total_cost), d.currency)
                    }
                    Some(ProviderDataState::Error(_)) => " ERR".to_string(),
                    None => String::new(),
                }
            };
            (format!(" {}{} ", name, badge), i == app.active_tab)
        })
        .collect();

    let inner_width = area.width.saturating_sub(2) as usize;
    let divider = " | ";
    let div_len = divider.len();
    let tab_widths: Vec<usize> = tab_labels.iter().map(|(l, _)| l.len() + div_len).collect();
    let total_width: usize = tab_widths.iter().sum::<usize>().saturating_sub(div_len);

    let (vis_start, vis_end, has_left, has_right) = if total_width <= inner_width {
        (0, tab_labels.len(), false, false)
    } else {
        let active = app.active_tab;
        let avail = inner_width.saturating_sub(8);
        let mut start = active;
        let mut end = active + 1;
        let mut used = tab_widths[active].saturating_sub(div_len);
        loop {
            let mut expanded = false;
            if end < tab_labels.len() {
                let w = tab_widths[end];
                if used + w <= avail {
                    used += w;
                    end += 1;
                    expanded = true;
                }
            }
            if start > 0 {
                let w = tab_widths[start - 1];
                if used + w <= avail {
                    used += w;
                    start -= 1;
                    expanded = true;
                }
            }
            if !expanded {
                break;
            }
        }
        (start, end, start > 0, end < tab_labels.len())
    };

    let mut spans: Vec<Span<'static>> = Vec::new();
    if has_left {
        spans.push(Span::styled(
            "<< ".to_string(),
            Style::default().fg(C_TEXT).add_modifier(Modifier::DIM),
        ));
    }
    for (i, (label, is_active)) in tab_labels
        .iter()
        .enumerate()
        .skip(vis_start)
        .take(vis_end - vis_start)
    {
        let style = if *is_active {
            Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(C_TEXT).add_modifier(Modifier::DIM)
        };
        spans.push(Span::styled(label.clone(), style));
        if i < vis_end - 1 {
            spans.push(Span::styled(
                divider.to_string(),
                Style::default().fg(C_BORDER),
            ));
        }
    }
    if has_right {
        spans.push(Span::styled(
            " >>".to_string(),
            Style::default().fg(C_TEXT).add_modifier(Modifier::DIM),
        ));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_BORDER));

    frame.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
}

// ── Summary (Logo + Bar Chart + Breakdown) ──────────────────────────────

fn render_summary(frame: &mut Frame, app: &App, area: Rect) {
    render_chart_panel(frame, app, area);
}

fn render_chart_panel(frame: &mut Frame, app: &App, area: Rect) {
    let summaries = app.cost_summary();
    let errors = app.loading_errors();
    let totals = app.totals_by_currency();
    let prev_totals = app.prev_totals_by_currency();
    let prev_map: std::collections::HashMap<String, f64> = prev_totals.into_iter().collect();

    let inner_w = area.width.saturating_sub(4) as usize;
    let inner_h = area.height.saturating_sub(2) as usize; // available content lines

    let mut lines: Vec<Line<'static>> = Vec::new();

    // -- Totals line --
    let mut total_spans: Vec<Span<'static>> = Vec::new();
    for (currency, cost) in &totals {
        if !total_spans.is_empty() {
            total_spans.push(Span::styled(
                "  +  ".to_string(),
                Style::default().fg(C_TEXT).add_modifier(Modifier::DIM),
            ));
        }
        total_spans.push(Span::styled(
            format_cost(*cost),
            Style::default().fg(C_TOTAL).add_modifier(Modifier::BOLD),
        ));
        total_spans.push(Span::styled(
            format!(" {}", currency),
            Style::default().fg(C_TEXT).add_modifier(Modifier::DIM),
        ));
        if let Some(&prev) = prev_map.get(currency) {
            if prev > 0.0 {
                let pct = (cost - prev) / prev * 100.0;
                let (arrow, color) = if pct > 0.0 {
                    ("^", C_UP)
                } else if pct < 0.0 {
                    ("v", C_DOWN)
                } else {
                    ("-", C_TEXT)
                };
                total_spans.push(Span::styled(
                    format!(" {}{:.1}%", arrow, pct.abs()),
                    Style::default().fg(color),
                ));
            }
        }
    }
    if total_spans.is_empty() && app.is_any_loading() {
        total_spans.push(Span::styled(
            format!(" {} Fetching costs...", app.spinner_char()),
            Style::default().fg(C_ACCENT),
        ));
    }
    lines.push(Line::from(total_spans));
    lines.push(Line::raw(""));

    // -- Bar chart per provider --
    let max_cost = summaries.iter().map(|(_, c, _)| *c).fold(0.0_f64, f64::max);

    // Determine if we need compact mode (2 providers per line)
    let loading_count = app
        .data
        .values()
        .filter(|s| matches!(s, ProviderDataState::Loading))
        .count();
    let total_items = summaries.len() + errors.len() + loading_count;
    let lines_for_header = 2; // total + blank
    let avail_for_items = inner_h.saturating_sub(lines_for_header);
    let compact = total_items > avail_for_items;

    if compact {
        // Compact: two providers per line, no bar chart
        let half_w = inner_w / 2;
        let mut pairs: Vec<Vec<(String, f64, String)>> = Vec::new();
        let mut row = Vec::new();
        for item in &summaries {
            row.push(item.clone());
            if row.len() == 2 {
                pairs.push(row.clone());
                row.clear();
            }
        }
        if !row.is_empty() {
            pairs.push(row);
        }

        for pair in &pairs {
            let mut spans: Vec<Span<'static>> = Vec::new();
            for (j, (provider, cost, currency)) in pair.iter().enumerate() {
                let color = provider_color(provider);
                let pct = if max_cost > 0.0 {
                    cost / max_cost * 100.0
                } else {
                    0.0
                };
                let col_w = if j == 0 { half_w } else { inner_w - half_w };
                let label = format!(
                    " {:<10} {:>10} {} {:>3.0}%",
                    truncate_str(provider, 10),
                    format_cost(*cost),
                    currency,
                    pct
                );
                let padded = format!("{:<width$}", label, width = col_w);
                spans.push(Span::styled(padded, Style::default().fg(color)));
            }
            lines.push(Line::from(spans));
        }
    } else {
        // Normal: one provider per line with bar chart
        let label_w = 13;
        let cost_w = 18;
        let bar_max = inner_w.saturating_sub(label_w + cost_w + 4);

        for (provider, cost, currency) in &summaries {
            let bar_len = if max_cost > 0.0 {
                ((cost / max_cost) * bar_max as f64).round() as usize
            } else {
                0
            };
            let pct = if max_cost > 0.0 {
                cost / max_cost * 100.0
            } else {
                0.0
            };
            let color = provider_color(provider);
            let bar: String = "█".repeat(bar_len);
            let pad: String = " ".repeat(bar_max.saturating_sub(bar_len));

            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {:<width$}", provider, width = label_w),
                    Style::default().fg(color),
                ),
                Span::styled(bar, Style::default().fg(color)),
                Span::styled(pad, Style::default()),
                Span::styled(
                    format!(" {:>10} {:<3}", format_cost(*cost), currency),
                    Style::default().fg(C_TEXT),
                ),
                Span::styled(
                    format!(" {:>3.0}%", pct),
                    Style::default().fg(C_TEXT).add_modifier(Modifier::DIM),
                ),
            ]));
        }
    }

    // Loading
    for (provider, state) in &app.data {
        if matches!(state, ProviderDataState::Loading) {
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {:<13}", provider),
                    Style::default().fg(C_TEXT).add_modifier(Modifier::DIM),
                ),
                Span::styled(
                    format!("{} loading...", app.spinner_char()),
                    Style::default().fg(C_TEXT).add_modifier(Modifier::DIM),
                ),
            ]));
        }
    }

    // Errors
    for (provider, err) in &errors {
        lines.push(Line::from(vec![
            Span::styled(format!(" {:<13}", provider), Style::default().fg(C_ERR)),
            Span::styled(truncate_str(err, 50), Style::default().fg(C_ERR)),
        ]));
    }

    // Build title: " CLOUD BILLING v0.1.0 | < 2026-03 > | Refreshed 12s ago "
    let current_month = chrono::Local::now().format("%Y-%m").to_string();
    let month_tag = if app.billing_cycle == current_month {
        " (current)"
    } else {
        ""
    };
    let status_str = if app.is_any_loading() {
        format!(
            " {} Loading {}/{} ",
            app.spinner_char(),
            app.loaded_count(),
            app.total_providers()
        )
    } else if let Some(t) = app.last_refresh {
        let s = t.elapsed().as_secs();
        if s < 60 {
            format!(" {}s ago ", s)
        } else {
            format!(" {}m ago ", s / 60)
        }
    } else {
        String::new()
    };

    let title = Line::from(vec![
        Span::styled(
            " CLOUD BILLING ",
            Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("v{}", cbilling::VERSION),
            Style::default().fg(C_TEXT).add_modifier(Modifier::DIM),
        ),
        Span::styled(
            " | < ",
            Style::default().fg(C_TEXT).add_modifier(Modifier::DIM),
        ),
        Span::styled(
            format!("{}{}", app.billing_cycle, month_tag),
            Style::default().fg(C_TEXT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " > |",
            Style::default().fg(C_TEXT).add_modifier(Modifier::DIM),
        ),
        Span::styled(
            status_str,
            Style::default().fg(C_TEXT).add_modifier(Modifier::DIM),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_BORDER))
        .title(title)
        .padding(Padding::horizontal(1));

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

// ── Search Bar ──────────────────────────────────────────────────────────

fn render_search_bar(frame: &mut Frame, app: &App, area: Rect) {
    let is_active = app.input_mode == InputMode::Search;
    let cursor = if is_active { "_" } else { "" };
    let line = Line::from(vec![
        Span::styled(
            " / ".to_string(),
            Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{}{}", app.search_query, cursor),
            Style::default().fg(C_TEXT),
        ),
        if !app.search_query.is_empty() {
            Span::styled(
                format!("  ({} results)", app.current_products().len()),
                Style::default().fg(C_TEXT).add_modifier(Modifier::DIM),
            )
        } else {
            Span::styled(
                "  type to filter...".to_string(),
                Style::default().fg(C_TEXT).add_modifier(Modifier::DIM),
            )
        },
    ]);

    let border_color = if is_active { C_ACCENT } else { C_BORDER };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(" Search ", Style::default().fg(C_TITLE)))
        .style(Style::default().bg(C_SEARCH_BG));

    frame.render_widget(Paragraph::new(line).block(block), area);
}

// ── Table ───────────────────────────────────────────────────────────────

fn render_table(frame: &mut Frame, app: &mut App, area: Rect) {
    let products = app.current_products();

    let sort_col = app.sort_column;
    let sort_asc = app.sort_ascending;
    let arrow = |col: SortColumn| -> &'static str {
        if sort_col == col {
            if sort_asc {
                " ▲"
            } else {
                " ▼"
            }
        } else {
            ""
        }
    };
    let hdr_style = |col: SortColumn| -> Style {
        if sort_col == col {
            Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(C_TITLE).add_modifier(Modifier::BOLD)
        }
    };

    let dim = Style::default().fg(C_TEXT).add_modifier(Modifier::DIM);

    let header = Row::new(vec![
        Cell::from(" #").style(hdr_style(SortColumn::Product)),
        Cell::from(format!(" Product{}", arrow(SortColumn::Product)))
            .style(hdr_style(SortColumn::Product)),
        Cell::from(format!(" Code{}", arrow(SortColumn::Code))).style(hdr_style(SortColumn::Code)),
        Cell::from(format!("       Cost{}", arrow(SortColumn::Cost)))
            .style(hdr_style(SortColumn::Cost)),
        Cell::from(format!("  MoM{}", arrow(SortColumn::MoM))).style(hdr_style(SortColumn::MoM)),
        Cell::from(" Qty").style(Style::default().fg(C_TITLE).add_modifier(Modifier::BOLD)),
        Cell::from(" Region").style(Style::default().fg(C_TITLE).add_modifier(Modifier::BOLD)),
    ])
    .height(1);

    let rows: Vec<Row> = products
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let cost_color = if p.cost > 10000.0 {
                C_COST_HIGH
            } else if p.cost > 1000.0 {
                C_COST_MID
            } else {
                C_COST_LOW
            };

            let (mom_str, mom_color) = match p.mom_change {
                Some(pct) if pct == f64::INFINITY => (" NEW".to_string(), C_ACCENT),
                Some(pct) if pct > 20.0 => (format!("▲▲{:+.0}%", pct), C_UP),
                Some(pct) if pct > 0.0 => (format!(" ▲{:+.0}%", pct), C_UP),
                Some(pct) if pct < -20.0 => (format!("▼▼{:.0}%", pct), C_DOWN),
                Some(pct) if pct < 0.0 => (format!(" ▼{:.0}%", pct), C_DOWN),
                Some(_) => ("  ─ 0%".to_string(), C_TEXT),
                None => ("     -".to_string(), C_TEXT),
            };

            // Qty with change indicator
            let qty_str = match (p.count, p.count_change) {
                (Some(c), Some(d)) if d > 0 => format!("{:>4} +{}", c, d),
                (Some(c), Some(d)) if d < 0 => format!("{:>4} {}", c, d),
                (Some(c), _) => format!("{:>4}", c),
                _ => "   -".to_string(),
            };
            let qty_color = match p.count_change {
                Some(d) if d > 0 => C_UP,
                Some(d) if d < 0 => C_DOWN,
                _ => C_TEXT,
            };

            // Expand indicator
            let expand_marker = if !p.is_sub_row && !p.region_details.is_empty() {
                if app.expanded.contains(&p.product_code) {
                    "▼"
                } else {
                    "▶"
                }
            } else {
                " "
            };

            let name_style = if p.is_sub_row {
                dim
            } else {
                Style::default().fg(C_TEXT)
            };

            Row::new(vec![
                Cell::from(format!(
                    "{:>3}",
                    if p.is_sub_row {
                        String::new()
                    } else {
                        format!("{}", i + 1)
                    }
                ))
                .style(dim),
                Cell::from(format!("{}{}", expand_marker, &p.product_name)).style(name_style),
                Cell::from(format!(" {}", &p.product_code)).style(dim),
                Cell::from(format!("{:>12}", format_cost(p.cost)))
                    .style(Style::default().fg(cost_color)),
                Cell::from(format!("{:>7}", mom_str)).style(Style::default().fg(mom_color)),
                Cell::from(qty_str).style(Style::default().fg(qty_color)),
                Cell::from(format!(" {}", &p.regions_display)).style(dim),
            ])
        })
        .collect();

    let total = products.len();
    let selected = app.table_state.selected().unwrap_or(0);

    let widths = [
        Constraint::Length(4),  // #
        Constraint::Fill(4),    // Product
        Constraint::Fill(2),    // Code
        Constraint::Length(13), // Cost
        Constraint::Length(8),  // MoM
        Constraint::Length(9),  // Qty
        Constraint::Fill(2),    // Region
    ];

    let title = format!(
        " Products ({}) [{}/{}] ",
        total,
        if total > 0 { selected + 1 } else { 0 },
        total
    );

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .bg(C_HIGHLIGHT_BG)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(C_BORDER))
                .title(Span::styled(
                    title,
                    Style::default().fg(C_TITLE).add_modifier(Modifier::BOLD),
                )),
        );

    frame.render_stateful_widget(table, area, &mut app.table_state);

    if total > 0 {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("│"))
            .thumb_symbol("█");
        let mut scrollbar_state = ScrollbarState::new(total).position(selected);
        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

// ── Footer ──────────────────────────────────────────────────────────────

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let items: &[(&str, &str)] = if app.input_mode == InputMode::Search {
        &[("Enter", "Confirm"), ("Esc", "Cancel"), ("Type", "Filter")]
    } else {
        &[
            ("j/k", "Nav"),
            ("^f/^b", "Page"),
            ("Tab", "Provider"),
            ("</>", "Month"),
            ("s/S", "Sort"),
            ("1-5", "Col"),
            ("/", "Search"),
            ("r", "Refresh"),
            ("?", "Help"),
            ("q", "Quit"),
        ]
    };

    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, (k, d)) in items.iter().enumerate() {
        spans.push(Span::styled(
            k.to_string(),
            Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {}", d),
            Style::default().fg(C_TEXT).add_modifier(Modifier::DIM),
        ));
        if i < items.len() - 1 {
            spans.push(Span::styled(
                " | ".to_string(),
                Style::default().fg(C_BORDER),
            ));
        }
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
        area,
    );
}

// ── Help Overlay ────────────────────────────────────────────────────────

fn render_help(frame: &mut Frame) {
    let popup = centered_rect(45, 70, frame.area());
    frame.render_widget(Clear, popup);

    let help = vec![
        Line::from(Span::styled(
            " Keyboard Shortcuts",
            Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        help_line("j / Down  ", "Move down"),
        help_line("k / Up    ", "Move up"),
        help_line("Ctrl+f    ", "Page down"),
        help_line("Ctrl+b    ", "Page up"),
        help_line("g         ", "Jump to top"),
        help_line("G         ", "Jump to bottom"),
        help_line("Tab / l   ", "Next provider"),
        help_line("S-Tab / h ", "Previous provider"),
        Line::raw(""),
        help_line("Left / m  ", "Previous month"),
        help_line("Right / M ", "Next month"),
        help_line("r         ", "Refresh all data"),
        help_line("/         ", "Search / filter products"),
        help_line("Esc       ", "Clear search filter"),
        Line::raw(""),
        help_line("s         ", "Cycle sort column"),
        help_line("S         ", "Toggle asc/desc"),
        help_line("1-5       ", "Sort by column directly"),
        Line::raw(""),
        help_line("?         ", "Toggle this help"),
        help_line("q / Ctrl+C", "Quit"),
        Line::raw(""),
        Line::styled(
            "  MoM: ▲▲ >+20%  ▲ up  ▼ down  ▼▼ >-20%",
            Style::default().fg(C_TEXT).add_modifier(Modifier::DIM),
        ),
        Line::raw(""),
        Line::styled(
            " Press any key to close",
            Style::default().fg(C_TEXT).add_modifier(Modifier::DIM),
        ),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(C_ACCENT))
        .title(Span::styled(
            " Help ",
            Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(Color::Reset))
        .padding(Padding::new(2, 2, 1, 1));

    frame.render_widget(
        Paragraph::new(help).block(block).wrap(Wrap { trim: false }),
        popup,
    );
}

fn help_line<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {:<12}", key), Style::default().fg(C_ACCENT)),
        Span::styled(desc, Style::default().fg(C_TEXT)),
    ])
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let v = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(v[1])[1]
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!(
            "{}...",
            s.chars()
                .take(max_len.saturating_sub(3))
                .collect::<String>()
        )
    }
}

fn format_cost(cost: f64) -> String {
    let s = format!("{:.2}", cost);
    let parts: Vec<&str> = s.split('.').collect();
    let int_part = parts[0];
    let dec_part = parts.get(1).unwrap_or(&"00");
    let negative = int_part.starts_with('-');
    let digits: String = int_part.chars().filter(|c| c.is_ascii_digit()).collect();
    let mut result = String::new();
    for (i, ch) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    let formatted: String = result.chars().rev().collect();
    if negative {
        format!("-{}.{}", formatted, dec_part)
    } else {
        format!("{}.{}", formatted, dec_part)
    }
}

fn provider_color(name: &str) -> Color {
    match name {
        "aliyun" => Color::Yellow,
        "aws" => Color::LightYellow,
        "tencentcloud" => Color::Blue,
        "volcengine" => Color::Cyan,
        "ucloud" => Color::Magenta,
        "gcp" => Color::Green,
        "cloudflare" => Color::LightYellow,
        _ => C_TEXT,
    }
}
