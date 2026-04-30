//! `html::tagstrip` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "html::tagstrip",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Remove all HTML tags from a string.",
            &["html::tagstrip string"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
