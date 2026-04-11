//! `tcltest::bytestring` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::bytestring",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Convert a string to its byte representation (Tcl < 9.0).",
            &["tcltest::bytestring string"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
