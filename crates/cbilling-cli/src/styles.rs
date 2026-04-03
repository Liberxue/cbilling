/// Semantic style system for the cbilling TUI.
///
/// All style functions return `ratatui::style::Style` values with semantic meaning,
/// making it easy to maintain a consistent look-and-feel across the application.
use ratatui::style::{Color, Modifier, Style};

// ── Base palette ─────────────────────────────────────────────────────────

const CYAN: Color = Color::Cyan;
const GREEN: Color = Color::Green;
const RED: Color = Color::Red;
const YELLOW: Color = Color::Yellow;
const DARK_GRAY: Color = Color::DarkGray;

// ── Semantic styles ──────────────────────────────────────────────────────

/// Primary accent color for interactive elements and active items.
#[inline]
pub fn accent() -> Style {
    Style::default().fg(CYAN)
}

/// Bold accent for titles and active tabs.
#[inline]
pub fn accent_bold() -> Style {
    Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
}

/// Default text — inherits terminal foreground.
#[inline]
pub fn text() -> Style {
    Style::default().fg(Color::Reset)
}

/// Dimmed text for secondary information.
#[inline]
pub fn dim() -> Style {
    Style::default()
        .fg(Color::Reset)
        .add_modifier(Modifier::DIM)
}

/// Bold text for emphasis.
#[inline]
pub fn bold() -> Style {
    Style::default()
        .fg(Color::Reset)
        .add_modifier(Modifier::BOLD)
}

/// Border style — uses terminal default.
#[inline]
pub fn border() -> Style {
    Style::default().fg(Color::Reset)
}

/// Active border (e.g. focused search bar).
#[inline]
pub fn border_active() -> Style {
    Style::default().fg(CYAN)
}

/// Section title style.
#[inline]
pub fn title() -> Style {
    Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
}

/// Error text.
#[inline]
pub fn error() -> Style {
    Style::default().fg(RED)
}

/// Total cost display.
#[inline]
pub fn total() -> Style {
    Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
}

/// Selected row highlight.
#[inline]
pub fn highlight() -> Style {
    Style::default()
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD)
}

// ── Cost color tiers ─────────────────────────────────────────────────────

/// Color for cost values based on magnitude.
pub fn cost_color(cost: f64) -> Style {
    if cost > 10_000.0 {
        Style::default().fg(RED)
    } else if cost > 1_000.0 {
        Style::default().fg(YELLOW)
    } else {
        Style::default().fg(GREEN)
    }
}

// ── MoM (month-over-month) indicators ────────────────────────────────────

/// Style for cost increase.
#[inline]
pub fn up() -> Style {
    Style::default().fg(RED)
}

/// Style for cost decrease.
#[inline]
pub fn down() -> Style {
    Style::default().fg(GREEN)
}

// ── Provider brand colors ────────────────────────────────────────────────

pub fn provider(name: &str) -> Style {
    let color = match name {
        "aliyun" => YELLOW,
        "aws" => Color::LightYellow,
        "tencentcloud" => Color::Blue,
        "volcengine" => CYAN,
        "ucloud" => Color::Magenta,
        "gcp" => GREEN,
        "cloudflare" => Color::LightYellow,
        _ => Color::Reset,
    };
    Style::default().fg(color)
}

// ── Status indicators ────────────────────────────────────────────────────

/// Online / ready status.
#[inline]
pub fn status_ok() -> Style {
    Style::default().fg(GREEN)
}

/// Offline / error status.
#[inline]
pub fn status_err() -> Style {
    Style::default().fg(RED)
}

/// Loading / pending status.
#[inline]
pub fn status_loading() -> Style {
    Style::default().fg(CYAN)
}

/// Dark gray for inactive elements.
#[inline]
pub fn dark_gray() -> Style {
    Style::default().fg(DARK_GRAY)
}
