/// ANSI color codes
pub enum Color {
    Red,      // error, failed, behind, deleted
    Green,    // success, succeeded, ahead, added, clean
    Yellow,   // warning, dirty, modified, dry-run
    Cyan,     // branch name, count
    Gray,     // dim text, date
    BrightRed,
    BrightGreen,
    BrightYellow,
    Bold,
}

impl Color {
    /// Returns the ANSI escape code prefix
    pub fn code(&self) -> &'static str {
        match self {
            Color::Red => "\x1b[31m",
            Color::Green => "\x1b[32m",
            Color::Yellow => "\x1b[33m",
            Color::Cyan => "\x1b[36m",
            Color::Gray => "\x1b[90m",
            Color::BrightRed => "\x1b[91m",
            Color::BrightGreen => "\x1b[92m",
            Color::BrightYellow => "\x1b[93m",
            Color::Bold => "\x1b[1m",
        }
    }

    /// Reset ANSI formatting
    pub fn reset() -> &'static str {
        "\x1b[0m"
    }
}

/// Wrap text with a single color
pub fn c(color: Color, text: &str) -> String {
    format!("{}{}{}", color.code(), text, Color::reset())
}
