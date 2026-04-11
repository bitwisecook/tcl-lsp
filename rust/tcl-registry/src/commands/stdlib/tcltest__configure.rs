//! `tcltest::configure` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::configure",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set tcltest configuration options.",
            &["tcltest::configure ?option? ?value option value ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
