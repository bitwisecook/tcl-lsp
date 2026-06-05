//! `focus` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "focus",
        dialects: Some(DialectSet::TK),
        arity: Arity::new(0, 2),
        hover: Some(HoverSnippet::brief(
            "Manage the input focus.",
            &["focus"],
            "F5",
        )),
        required_package: Some("Tk"),
        warn_missing_import: false,
        ..CommandSpec::DEFAULT
    }
}
