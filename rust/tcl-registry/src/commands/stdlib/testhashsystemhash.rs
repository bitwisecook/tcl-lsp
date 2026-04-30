//! `testhashsystemhash` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testhashsystemhash",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test system hash implementation.",
            &["testhashsystemhash"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
