/// ASCII art logo widget for cbilling branding.
use ratatui::{
    style::Modifier,
    text::{Line, Span},
};

use crate::styles;

/// cbilling ASCII art logo lines.
const LOGO: &[&str] = &[
    r"        __    _ _____            ",
    r"  _____/ /_  (_) / (_)___  ____ _",
    r" / ___/ __ \/ / / / / __ \/ __ `/",
    r"/ /__/ /_/ / / / / / / / / /_/ / ",
    r"\___/_.___/_/_/_/_/_/ /_/\__, /  ",
    r"                        /____/   ",
];

/// Returns the logo as styled `Line`s for rendering.
pub fn logo_lines() -> Vec<Line<'static>> {
    LOGO.iter()
        .map(|line| {
            Line::from(Span::styled(
                line.to_string(),
                styles::accent().add_modifier(Modifier::BOLD),
            ))
        })
        .collect()
}

/// Returns the logo width (max line length).
pub fn logo_width() -> u16 {
    LOGO.iter().map(|l| l.len()).max().unwrap_or(0) as u16
}

/// Returns the logo height (number of lines).
pub fn logo_height() -> u16 {
    LOGO.len() as u16
}
