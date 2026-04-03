/// Help overlay popup with keyboard shortcuts reference.
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap},
    Frame,
};

use crate::styles;
use crate::widgets::logo;

/// Render the help popup overlay.
pub fn render(frame: &mut Frame) {
    let popup = centered_rect(50, 80, frame.area());
    frame.render_widget(Clear, popup);

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Logo at top of help
    for l in logo::logo_lines() {
        lines.push(l);
    }
    lines.push(Line::raw(""));

    // Section: Navigation
    lines.push(section_title("Navigation"));
    lines.push(key_line("j / ↓     ", "Move down"));
    lines.push(key_line("k / ↑     ", "Move up"));
    lines.push(key_line("Ctrl+f    ", "Page down"));
    lines.push(key_line("Ctrl+b    ", "Page up"));
    lines.push(key_line("g         ", "Jump to top"));
    lines.push(key_line("G         ", "Jump to bottom"));
    lines.push(key_line("Mouse     ", "Scroll with wheel"));
    lines.push(Line::raw(""));

    // Section: Tabs & Time
    lines.push(section_title("Tabs & Time"));
    lines.push(key_line("Tab / l   ", "Next provider"));
    lines.push(key_line("S-Tab / h ", "Previous provider"));
    lines.push(key_line("← / m     ", "Previous month"));
    lines.push(key_line("→ / M     ", "Next month"));
    lines.push(Line::raw(""));

    // Section: Sort & Filter
    lines.push(section_title("Sort & Filter"));
    lines.push(key_line("s         ", "Cycle sort column"));
    lines.push(key_line("S         ", "Toggle sort direction"));
    lines.push(key_line("1-5       ", "Sort by column #"));
    lines.push(key_line("/         ", "Search / filter"));
    lines.push(key_line("Enter     ", "Expand region details"));
    lines.push(key_line("Esc       ", "Clear filter"));
    lines.push(Line::raw(""));

    // Section: Actions
    lines.push(section_title("Actions"));
    lines.push(key_line("r         ", "Refresh all data"));
    lines.push(key_line("?         ", "Toggle this help"));
    lines.push(key_line("q / Ctrl+C", "Quit"));
    lines.push(Line::raw(""));

    // Legend
    lines.push(Line::from(vec![
        Span::styled("  MoM: ", styles::dim()),
        Span::styled("▲▲", styles::up()),
        Span::styled(" >+20%  ", styles::dim()),
        Span::styled("▲", styles::up()),
        Span::styled(" up  ", styles::dim()),
        Span::styled("▼", styles::down()),
        Span::styled(" down  ", styles::dim()),
        Span::styled("▼▼", styles::down()),
        Span::styled(" >-20%", styles::dim()),
    ]));

    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        " Press any key to close",
        styles::dim().add_modifier(Modifier::ITALIC),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(styles::border_active())
        .title(Span::styled(" Help ", styles::accent_bold()))
        .title_alignment(Alignment::Center)
        .padding(Padding::new(2, 2, 1, 1));

    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn section_title(title: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  ── {} ", title),
            styles::accent().add_modifier(Modifier::BOLD),
        ),
        Span::styled("─".repeat(20), styles::dark_gray()),
    ])
}

fn key_line<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {:<12}", key), styles::accent()),
        Span::styled(desc, styles::text()),
    ])
}

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
