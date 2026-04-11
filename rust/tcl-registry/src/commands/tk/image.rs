//! `image` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "image",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate images.",
            &["image create type ?name? ?option value ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
