//! `scrollbar` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "scrollbar",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate a scrollbar widget.",
            &["scrollbar pathName ?option value ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
