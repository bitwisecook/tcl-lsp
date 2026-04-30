//! `grab` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "grab",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Confine pointer and keyboard events to a window sub-tree.",
            &["grab ?-global? window"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
