//! `tcltest::normalizeMsg` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::normalizeMsg",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Normalise an error message for comparison (lowercase, strip trailing newline).",
            &["tcltest::normalizeMsg msg"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
