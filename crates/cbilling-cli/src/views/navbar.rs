/// Provider tab navigation bar.
///
/// Renders a horizontal tab strip with provider names and cost badges.
/// Supports scrollable tabs when there are too many providers to fit.
use ratatui::{
    layout::Rect,
    layout::{Constraint, Layout},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::{App, ProviderDataState};
use crate::styles;
use crate::views::summary::format_cost;

/// Render the provider tab bar.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
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
    let divider = " │ ";
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
        spans.push(Span::styled("◀ ".to_string(), styles::dim()));
    }
    for (i, (label, is_active)) in tab_labels
        .iter()
        .enumerate()
        .skip(vis_start)
        .take(vis_end - vis_start)
    {
        let style = if *is_active {
            styles::accent_bold()
        } else {
            styles::dim()
        };
        spans.push(Span::styled(label.clone(), style));
        if i < vis_end - 1 {
            spans.push(Span::styled(divider.to_string(), styles::dark_gray()));
        }
    }
    if has_right {
        spans.push(Span::styled(" ▶".to_string(), styles::dim()));
    }

    // Right side: version + status
    let right_info = format!(" v{} ", cbilling::VERSION);

    let cols = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(right_info.len() as u16 + 2),
    ])
    .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border());

    frame.render_widget(Paragraph::new(Line::from(spans)).block(block), cols[0]);

    // Version badge on right
    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border());
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(right_info, styles::dim()))).block(right_block),
        cols[1],
    );
}
