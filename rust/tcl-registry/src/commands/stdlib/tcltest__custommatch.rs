//! `tcltest::customMatch` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::customMatch",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Register a custom matching command for test results.",
            &["tcltest::customMatch mode command"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
