//! `testpurebytesobj` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testpurebytesobj",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test pure-bytes Tcl_Obj operations.",
            &["testpurebytesobj"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
