//! `tcl_startOfNextWord` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl_startOfNextWord",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Return the index of the first start-of-word after *start* in *str*.",
            &["tcl_startOfNextWord str start"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
