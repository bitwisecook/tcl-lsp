//! `testevalex` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testevalex",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_EvalEx.",
            &["testevalex"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
