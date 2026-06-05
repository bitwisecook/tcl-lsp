//! `wm` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "wm",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Communicate with the window manager.",
            &["wm aspect window ?minNumer minDenom maxNumer maxDenom?"],
            "F5",
        )),
        required_package: Some("Tk"),
        warn_missing_import: false,
        ..CommandSpec::DEFAULT
    }
}
