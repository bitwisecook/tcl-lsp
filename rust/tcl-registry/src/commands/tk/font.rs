//! `font` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "font",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and inspect fonts.",
            &["font actual font ?-displayof window? ?option?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
