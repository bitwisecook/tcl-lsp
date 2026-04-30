//! `tcltest::threadReap` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::threadReap",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Terminate all threads except the main thread and return the thread count.",
            &["tcltest::threadReap"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
