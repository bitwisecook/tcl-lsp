//! `tcl_endOfWord` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl_endOfWord",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Return the index of the first end-of-word after *start* in *str*.",
            &["tcl_endOfWord str start"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
