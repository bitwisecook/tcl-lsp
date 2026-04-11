//! `tcl_startOfPreviousWord` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl_startOfPreviousWord",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Return the index of the first start-of-word before *start* in *str*.",
            &["tcl_startOfPreviousWord str start"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
