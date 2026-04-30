//! `raise` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "raise",
        dialects: Some(DialectSet::TK),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Raise a window's position in the stacking order.",
            &["raise window ?aboveThis?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
