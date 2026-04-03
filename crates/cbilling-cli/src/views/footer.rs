/// Footer status bar with keyboard hints and connection status.
///
/// Inspired by longbridge-terminal's footer with status indicators.
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, InputMode};
use crate::styles;
use crate::widgets::loading;

/// Render the footer bar.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::horizontal([Constraint::Min(0), Constraint::Length(6)]).split(area);

    // Left: keyboard shortcuts
    let items: &[(&str, &str)] = if app.input_mode == InputMode::Search {
        &[("Enter", "OK"), ("Esc", "Cancel"), ("Type", "Filter")]
    } else {
        &[
            ("j/k", "Nav"),
            ("^f/^b", "Page"),
            ("Tab", "Tab"),
            ("◀/▶", "Month"),
            ("s", "Sort"),
            ("/", "Search"),
            ("r", "Refresh"),
            ("?", "Help"),
            ("q", "Quit"),
        ]
    };

    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, (k, d)) in items.iter().enumerate() {
        spans.push(Span::styled(
            format!(" {}", k),
            styles::accent().add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(format!(" {}", d), styles::dim()));
        if i < items.len() - 1 {
            spans.push(Span::styled(" │".to_string(), styles::dark_gray()));
        }
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Left),
        cols[0],
    );

    // Right: connection status indicator
    let indicator = loading::status_indicator(app.loaded_count(), app.total_providers());
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::raw(" "), indicator])).alignment(Alignment::Right),
        cols[1],
    );
}
