/// Full-screen loading overlay widget.
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph},
    Frame,
};

use crate::styles;
use crate::widgets::logo;

/// Render a loading splash screen with logo and progress.
pub fn render_loading(frame: &mut Frame, loaded: usize, total: usize, spinner: char) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    // Center the content vertically
    let content_height = logo::logo_height() + 6; // logo + spacing + progress + status
    let v = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(content_height),
        Constraint::Min(0),
    ])
    .split(area);

    let center = v[1];
    let content_width = logo::logo_width() + 8;
    let h = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(content_width.min(area.width)),
        Constraint::Min(0),
    ])
    .split(center);

    let box_area = h[1];

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Logo
    lines.extend(logo::logo_lines());
    lines.push(Line::raw(""));

    // Progress bar
    let pct = if total > 0 {
        (loaded as f64 / total as f64 * 100.0) as u16
    } else {
        0
    };
    let bar_w = (box_area.width.saturating_sub(8)) as usize;
    let filled = (bar_w as f64 * loaded as f64 / total.max(1) as f64) as usize;
    let empty = bar_w.saturating_sub(filled);
    let bar = format!("  [{}{}] {}%", "█".repeat(filled), "░".repeat(empty), pct);
    lines.push(Line::from(Span::styled(bar, styles::accent())));
    lines.push(Line::raw(""));

    // Status text
    let status = format!("  {} Loading providers... {}/{}", spinner, loaded, total);
    lines.push(Line::from(Span::styled(
        status,
        styles::dim().add_modifier(Modifier::ITALIC),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border_active())
        .padding(Padding::new(2, 2, 1, 1));

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .block(block),
        box_area,
    );
}

/// Smaller inline loading indicator for panels.
#[allow(dead_code)]
pub fn loading_line(spinner: char, label: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!(" {} ", spinner),
            styles::status_loading().add_modifier(Modifier::BOLD),
        ),
        Span::styled(label.to_string(), styles::dim()),
    ])
}

/// Connection status indicator (inspired by longbridge's ■□ pattern).
pub fn status_indicator(loaded: usize, total: usize) -> Span<'static> {
    if loaded == 0 && total > 0 {
        Span::styled("□□□".to_string(), styles::status_err())
    } else if loaded < total {
        Span::styled("□□■".to_string(), styles::status_loading())
    } else {
        Span::styled("■■■".to_string(), styles::status_ok())
    }
}

fn _centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let v = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(height),
        Constraint::Min(0),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(width),
        Constraint::Min(0),
    ])
    .split(v[1])[1]
}
