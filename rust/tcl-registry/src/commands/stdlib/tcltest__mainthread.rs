//! `tcltest::mainThread` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::mainThread",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set the main thread ID.",
            &["tcltest::mainThread ?id?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
