//! `canvas` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "canvas",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate a canvas widget.",
            &["canvas pathName ?option value ...?"],
            "F5",
        )),
        required_package: Some("Tk"),
        warn_missing_import: false,
        ..CommandSpec::DEFAULT
    }
}
