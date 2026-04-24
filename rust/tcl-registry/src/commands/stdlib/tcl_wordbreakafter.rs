//! `tcl_wordBreakAfter` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl_wordBreakAfter",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Return the index of the first word boundary after *start* in *str*.",
            &["tcl_wordBreakAfter str start"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
