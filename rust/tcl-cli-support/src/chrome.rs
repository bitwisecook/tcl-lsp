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
/// styling, leaving the exact `error: {msg}` line.
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

/// Render a table with the project's house style (rounded borders) from a
/// header row and body rows.
///
/// The single entry point for tabular verbs (`stats`, `registry`, `pkg list`,
/// …) so every table looks the same. Pass the column headers and one iterator
/// of cells per row; each cell only needs to be `Into<String>`.
#[must_use]
pub fn render_table(
    headers: impl IntoIterator<Item = impl Into<String>>,
    rows: impl IntoIterator<Item = impl IntoIterator<Item = impl Into<String>>>,
) -> String {
    use tabled::builder::Builder;
    use tabled::settings::Style as TableStyle;

    let mut builder = Builder::new();
    builder.push_record(headers);
    for row in rows {
        builder.push_record(row);
    }
    builder.build().with(TableStyle::rounded()).to_string()
}

#[cfg(test)]
mod tests {
    use super::{dim_style, error_style, heading_style, render_table, success_style, warn_style};

    #[test]
    fn render_table_emits_header_rows_and_border() {
        let table = render_table(["name", "count"], [["pool", "3"], ["node", "5"]]);
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
