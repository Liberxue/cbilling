/// Main UI orchestrator.
///
/// Delegates rendering to view and widget modules. This file only handles
/// layout computation and dispatching to the appropriate view renderers.
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Table,
    },
    Frame,
};

use crate::app::{App, InputMode, ProviderDataState, SortColumn};
use crate::styles;
use crate::views;
use crate::widgets;

pub fn draw(frame: &mut Frame, app: &mut App) {
    // Show loading splash on initial load (no data yet)
    let all_loading = app
        .data
        .values()
        .all(|s| matches!(s, ProviderDataState::Loading))
        && !app.data.is_empty();
    if all_loading {
        widgets::loading::render_loading(
            frame,
            app.loaded_count(),
            app.total_providers(),
            app.spinner_char(),
        );
        return;
    }

    let has_search = app.input_mode == InputMode::Search || !app.search_query.is_empty();

    // Dynamic summary height
    let provider_count = app.cost_summary().len()
        + app.loading_errors().len()
        + app
            .data
            .values()
            .filter(|s| matches!(s, ProviderDataState::Loading))
            .count();
    let content_lines = 2 + provider_count;
    let summary_h = (content_lines as u16 + 2)
        .max(5)
        .min(frame.area().height / 2)
        .min(18);

    let mut constraints = vec![
        Constraint::Length(3),         // navbar
        Constraint::Length(summary_h), // summary
    ];
    if has_search {
        constraints.push(Constraint::Length(3)); // search bar
    }
    constraints.push(Constraint::Min(6)); // table
    constraints.push(Constraint::Length(1)); // footer

    let outer = Layout::vertical(constraints).split(frame.area());

    let mut idx = 0;
    views::navbar::render(frame, app, outer[idx]);
    idx += 1;
    views::summary::render(frame, app, outer[idx]);
    idx += 1;
    if has_search {
        render_search_bar(frame, app, outer[idx]);
        idx += 1;
    }
    render_table(frame, app, outer[idx]);
    idx += 1;
    views::footer::render(frame, app, outer[idx]);

    if app.show_help {
        views::help::render(frame);
    }
}

// ── Search Bar ──────────────────────────────────────────────────────────

fn render_search_bar(frame: &mut Frame, app: &App, area: Rect) {
    let is_active = app.input_mode == InputMode::Search;
    let cursor = if is_active { "█" } else { "" };
    let line = Line::from(vec![
        Span::styled(" / ".to_string(), styles::accent_bold()),
        Span::styled(format!("{}{}", app.search_query, cursor), styles::text()),
        if !app.search_query.is_empty() {
            Span::styled(
                format!("  ({} results)", app.current_products().len()),
                styles::dim(),
            )
        } else {
            Span::styled("  type to filter...".to_string(), styles::dim())
        },
    ]);

    let border_style = if is_active {
        styles::border_active()
    } else {
        styles::border()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(Span::styled(" Search ", styles::title()));

    frame.render_widget(ratatui::widgets::Paragraph::new(line).block(block), area);
}

// ── Product Table ───────────────────────────────────────────────────────

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
    let hdr_style = |col: SortColumn| {
        if sort_col == col {
            styles::accent_bold()
        } else {
            styles::title()
        }
    };

    let header = Row::new(vec![
        Cell::from(" #").style(hdr_style(SortColumn::Product)),
        Cell::from(format!(" Product{}", arrow(SortColumn::Product)))
            .style(hdr_style(SortColumn::Product)),
        Cell::from(format!(" Code{}", arrow(SortColumn::Code))).style(hdr_style(SortColumn::Code)),
        Cell::from(format!("       Cost{}", arrow(SortColumn::Cost)))
            .style(hdr_style(SortColumn::Cost)),
        Cell::from(format!("  MoM{}", arrow(SortColumn::MoM))).style(hdr_style(SortColumn::MoM)),
        Cell::from(" Qty").style(styles::title()),
        Cell::from(" Region").style(styles::title()),
    ])
    .height(1);

    let rows: Vec<Row> = products
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let cost_style = styles::cost_color(p.cost);

            let (mom_str, mom_style) = match p.mom_change {
                Some(pct) if pct == f64::INFINITY => (" NEW".to_string(), styles::accent()),
                Some(pct) if pct > 20.0 => (format!("▲▲{:+.0}%", pct), styles::up()),
                Some(pct) if pct > 0.0 => (format!(" ▲{:+.0}%", pct), styles::up()),
                Some(pct) if pct < -20.0 => (format!("▼▼{:.0}%", pct), styles::down()),
                Some(pct) if pct < 0.0 => (format!(" ▼{:.0}%", pct), styles::down()),
                Some(_) => ("  ─ 0%".to_string(), styles::text()),
                None => ("     -".to_string(), styles::text()),
            };

            let qty_str = match (p.count, p.count_change) {
                (Some(c), Some(d)) if d > 0 => format!("{:>4} +{}", c, d),
                (Some(c), Some(d)) if d < 0 => format!("{:>4} {}", c, d),
                (Some(c), _) => format!("{:>4}", c),
                _ => "   -".to_string(),
            };
            let qty_style = match p.count_change {
                Some(d) if d > 0 => styles::up(),
                Some(d) if d < 0 => styles::down(),
                _ => styles::text(),
            };

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
                styles::dim()
            } else {
                styles::text()
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
                .style(styles::dim()),
                Cell::from(format!("{}{}", expand_marker, &p.product_name)).style(name_style),
                Cell::from(format!(" {}", &p.product_code)).style(styles::dim()),
                Cell::from(format!("{:>12}", views::summary::format_cost(p.cost)))
                    .style(cost_style),
                Cell::from(format!("{:>7}", mom_str)).style(mom_style),
                Cell::from(qty_str).style(qty_style),
                Cell::from(format!(" {}", &p.regions_display)).style(styles::dim()),
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
        .row_highlight_style(styles::highlight())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(styles::border())
                .title(Span::styled(title, styles::title())),
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
