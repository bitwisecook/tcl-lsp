//! `testuniclass` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testuniclass",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Unicode character classification (9.0+).",
            &["testuniclass"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
