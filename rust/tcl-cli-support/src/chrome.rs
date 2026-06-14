//! Shared terminal "chrome" for the native CLIs — styled status / error output
//! and consistent tables.
//!
//! Styling goes through [`anstream`], which auto-detects whether the stream is
//! a terminal and honours `NO_COLOR` / `CLICOLOR_FORCE`, stripping ANSI when
//! the output is piped. Tables use [`tabled`] with one house style.
//!
//! IMPORTANT: chrome is for **stderr, error messages, and new decorative
//! surfaces only** — never the byte-parity verb *stdout*. Because anstream
//! keeps piped output plain, scripted use and the golden parity tests stay
//! byte-stable while interactive terminals gain colour.

use std::fmt::Display;
use std::io::Write;

use anstyle::{AnsiColor, Style};

pub use tabled::Tabled;

/// Style for error prefixes (red, bold).
#[must_use]
pub fn error_style() -> Style {
    Style::new().fg_color(Some(AnsiColor::Red.into())).bold()
}

/// Style for warnings (yellow).
#[must_use]
pub fn warn_style() -> Style {
    Style::new().fg_color(Some(AnsiColor::Yellow.into()))
}

/// Style for success / "ok" notes (green).
#[must_use]
pub fn success_style() -> Style {
    Style::new().fg_color(Some(AnsiColor::Green.into()))
}

/// Style for section headings (bold).
#[must_use]
pub fn heading_style() -> Style {
    Style::new().bold()
}

/// Style for de-emphasised text (dimmed).
#[must_use]
pub fn dim_style() -> Style {
    Style::new().dimmed()
}

/// Print `error: {msg}` to stderr with the prefix styled.
///
/// On a non-terminal stderr (pipes, the test harness) anstream strips the
/// styling, leaving the exact `error: {msg}` line the Python CLI emits.
pub fn eprint_error(msg: impl Display) {
    let s = error_style();
    let mut err = anstream::stderr();
    let _ = writeln!(err, "{}error:{} {msg}", s.render(), s.render_reset());
}

/// Print a styled status line to stderr (auto-plain when piped).
pub fn eprint_status(style: Style, msg: impl Display) {
    let mut err = anstream::stderr();
    let _ = writeln!(err, "{}{msg}{}", style.render(), style.render_reset());
}

/// Render `rows` as a table with the project's house style (rounded borders).
///
/// The single entry point for tabular verbs (`stats`, `registry`, `pkg list`,
/// …) so every table looks the same. Derive [`Tabled`] on the row type.
#[must_use]
pub fn render_table<T, I>(rows: I) -> String
where
    T: Tabled,
    I: IntoIterator<Item = T>,
{
    use tabled::settings::Style as TableStyle;
    tabled::Table::new(rows)
        .with(TableStyle::rounded())
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{dim_style, error_style, heading_style, render_table, success_style, warn_style};

    #[derive(tabled::Tabled)]
    struct Row {
        name: &'static str,
        count: usize,
    }

    #[test]
    fn render_table_emits_header_rows_and_border() {
        let table = render_table([
            Row {
                name: "pool",
                count: 3,
            },
            Row {
                name: "node",
                count: 5,
            },
        ]);
        assert!(table.contains("name"), "header present: {table}");
        assert!(
            table.contains("pool") && table.contains('5'),
            "rows present"
        );
        assert!(table.contains('─'), "rounded border drawn");
    }

    #[test]
    fn styles_construct() {
        // Smoke-check the palette builders don't panic and differ from plain.
        let plain = anstyle::Style::new();
        assert_ne!(error_style(), plain);
        assert_ne!(warn_style(), plain);
        assert_ne!(success_style(), plain);
        assert_ne!(heading_style(), plain);
        assert_ne!(dim_style(), plain);
    }
}
