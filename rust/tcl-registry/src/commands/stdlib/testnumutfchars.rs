//! `testnumutfchars` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testnumutfchars",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_NumUtfChars.",
            &["testnumutfchars"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
