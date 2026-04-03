/// Summary panel with cost breakdown bar chart.
///
/// Shows total costs, per-provider breakdown with horizontal bar charts,
/// loading indicators, and error states.
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
    Frame,
};

use crate::app::{App, ProviderDataState};
use crate::styles;

/// Render the summary panel.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let summaries = app.cost_summary();
    let errors = app.loading_errors();
    let totals = app.totals_by_currency();
    let prev_totals = app.prev_totals_by_currency();
    let prev_map: std::collections::HashMap<String, f64> = prev_totals.into_iter().collect();

    let inner_w = area.width.saturating_sub(4) as usize;
    let inner_h = area.height.saturating_sub(2) as usize;

    let mut lines: Vec<Line<'static>> = Vec::new();

    // -- Totals line --
    let mut total_spans: Vec<Span<'static>> = Vec::new();
    for (currency, cost) in &totals {
        if !total_spans.is_empty() {
            total_spans.push(Span::styled("  +  ".to_string(), styles::dim()));
        }
        total_spans.push(Span::styled(format_cost(*cost), styles::total()));
        total_spans.push(Span::styled(format!(" {}", currency), styles::dim()));
        if let Some(&prev) = prev_map.get(currency) {
            if prev > 0.0 {
                let pct = (cost - prev) / prev * 100.0;
                let (arrow, style) = if pct > 0.0 {
                    ("↑", styles::up())
                } else if pct < 0.0 {
                    ("↓", styles::down())
                } else {
                    ("─", styles::text())
                };
                total_spans.push(Span::styled(format!(" {}{:.1}%", arrow, pct.abs()), style));
            }
        }
    }
    if total_spans.is_empty() && app.is_any_loading() {
        total_spans.push(Span::styled(
            format!(" {} Fetching costs...", app.spinner_char()),
            styles::status_loading(),
        ));
    }
    lines.push(Line::from(total_spans));
    lines.push(Line::raw(""));

    // -- Bar chart per provider --
    let max_cost = summaries.iter().map(|(_, c, _)| *c).fold(0.0_f64, f64::max);

    let loading_count = app
        .data
        .values()
        .filter(|s| matches!(s, ProviderDataState::Loading))
        .count();
    let total_items = summaries.len() + errors.len() + loading_count;
    let lines_for_header = 2;
    let avail_for_items = inner_h.saturating_sub(lines_for_header);
    let compact = total_items > avail_for_items;

    if compact {
        render_compact(&mut lines, &summaries, max_cost, inner_w);
    } else {
        render_bars(&mut lines, &summaries, max_cost, inner_w);
    }

    // Loading indicators
    for (provider, state) in &app.data {
        if matches!(state, ProviderDataState::Loading) {
            lines.push(Line::from(vec![
                Span::styled(format!(" {:<13}", provider), styles::dim()),
                Span::styled(
                    format!("{} loading...", app.spinner_char()),
                    styles::status_loading(),
                ),
            ]));
        }
    }

    // Error lines
    for (provider, err) in &errors {
        lines.push(Line::from(vec![
            Span::styled(format!(" {:<13}", provider), styles::error()),
            Span::styled(truncate_str(err, 50), styles::error()),
        ]));
    }

    // Title bar
    let current_month = chrono::Local::now().format("%Y-%m").to_string();
    let month_tag = if app.billing_cycle == current_month {
        " (current)"
    } else {
        ""
    };
    let status_str = if app.is_any_loading() {
        format!(
            " {} {}/{}",
            app.spinner_char(),
            app.loaded_count(),
            app.total_providers()
        )
    } else if let Some(t) = app.last_refresh {
        let s = t.elapsed().as_secs();
        if s < 60 {
            format!(" {}s ago", s)
        } else {
            format!(" {}m ago", s / 60)
        }
    } else {
        String::new()
    };

    let title = Line::from(vec![
        Span::styled(" CLOUD BILLING ", styles::accent_bold()),
        Span::styled(" ◀ ".to_string(), styles::dim()),
        Span::styled(
            format!("{}{}", app.billing_cycle, month_tag),
            styles::bold(),
        ),
        Span::styled(" ▶ ".to_string(), styles::dim()),
        Span::styled(status_str, styles::dim()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border())
        .title(title)
        .padding(Padding::horizontal(1));

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_bars(
    lines: &mut Vec<Line<'static>>,
    summaries: &[(String, f64, String)],
    max_cost: f64,
    inner_w: usize,
) {
    let label_w = 13;
    let cost_w = 18;
    let bar_max = inner_w.saturating_sub(label_w + cost_w + 4);

    for (provider, cost, currency) in summaries {
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
        let style = styles::provider(provider);
        let bar: String = "█".repeat(bar_len);
        let pad: String = " ".repeat(bar_max.saturating_sub(bar_len));

        lines.push(Line::from(vec![
            Span::styled(format!(" {:<width$}", provider, width = label_w), style),
            Span::styled(bar, style),
            Span::styled(pad, styles::text()),
            Span::styled(
                format!(" {:>10} {:<3}", format_cost(*cost), currency),
                styles::text(),
            ),
            Span::styled(format!(" {:>3.0}%", pct), styles::dim()),
        ]));
    }
}

fn render_compact(
    lines: &mut Vec<Line<'static>>,
    summaries: &[(String, f64, String)],
    max_cost: f64,
    inner_w: usize,
) {
    let half_w = inner_w / 2;
    let mut pairs: Vec<Vec<(String, f64, String)>> = Vec::new();
    let mut row = Vec::new();
    for item in summaries {
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
            let style = styles::provider(provider);
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
            spans.push(Span::styled(padded, style));
        }
        lines.push(Line::from(spans));
    }
}

/// Format a cost value with thousands separators.
pub fn format_cost(cost: f64) -> String {
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
