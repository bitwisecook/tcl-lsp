//! CLI output helpers — colour, `--json` mode, status symbols.
//!
//! Centralises the conventions shared
//! by all `tcl pkg` / `tcl venv` subcommands: ANSI colour codes, the
//! check/cross/warning symbols, and the canonical JSON output mode.

use std::io::IsTerminal;

use serde_json::Value;

use crate::json::canonical_json;

const RESET: &str = "\x1b[0m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";

fn is_stdout_tty() -> bool {
    std::io::stdout().is_terminal()
}

/// Decide whether to emit ANSI colour codes.
///
/// `force` overrides auto-detection (`Some(true)` always, `Some(false)` never).
/// `None` auto-detects from the terminal and the `NO_COLOR` / `FORCE_COLOR`
/// environment variables.
#[must_use]
pub fn use_colour(force: Option<bool>) -> bool {
    if let Some(value) = force {
        value
    } else {
        if std::env::var_os("NO_COLOR").is_some() {
            return false;
        }
        if std::env::var_os("FORCE_COLOR").is_some() {
            return true;
        }
        is_stdout_tty()
    }
}

fn colourise(code: &str, text: &str, colour: bool) -> String {
    if colour {
        format!("{code}{text}{RESET}")
    } else {
        text.to_string()
    }
}

#[must_use]
pub fn ok(text: &str, colour: bool) -> String {
    colourise(GREEN, &format!("  \u{2713} {text}"), colour)
}

#[must_use]
pub fn fail(text: &str, colour: bool) -> String {
    colourise(RED, &format!("  \u{2717} {text}"), colour)
}

#[must_use]
pub fn warn(text: &str, colour: bool) -> String {
    colourise(YELLOW, &format!("  \u{26a0} {text}"), colour)
}

#[must_use]
pub fn info(text: &str, colour: bool) -> String {
    colourise(CYAN, text, colour)
}

#[must_use]
pub fn dim(text: &str, colour: bool) -> String {
    colourise(DIM, text, colour)
}

#[must_use]
pub fn bold(text: &str, colour: bool) -> String {
    colourise(BOLD, text, colour)
}

/// Render `data` as canonical JSON (sorted keys, 2-space indent) for the
/// `--json` output, with a trailing newline ready for `println!`-style
/// printing without doubling the newline.
#[must_use]
pub fn json_output(data: &Value) -> String {
    canonical_json(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbols_and_colour() {
        assert_eq!(ok("done", false), "  \u{2713} done");
        assert_eq!(fail("bad", false), "  \u{2717} bad");
        assert_eq!(warn("hmm", false), "  \u{26a0} hmm");
        assert!(ok("done", true).starts_with(GREEN));
        assert!(ok("done", true).ends_with(RESET));
    }

    #[test]
    fn colour_forcing() {
        assert!(use_colour(Some(true)));
        assert!(!use_colour(Some(false)));
    }
}
