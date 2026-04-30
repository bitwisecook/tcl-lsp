//! `tcl_wordBreakBefore` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl_wordBreakBefore",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Return the index of the first word boundary before *start* in *str*.",
            &["tcl_wordBreakBefore str start"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
