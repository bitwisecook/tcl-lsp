//! `lower` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lower",
        dialects: Some(DialectSet::TK),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Lower a window's position in the stacking order.",
            &["lower window ?belowThis?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
