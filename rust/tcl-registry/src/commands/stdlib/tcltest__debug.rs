//! `tcltest::debug` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::debug",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set debug level.  Deprecated: use ``configure -debug``.",
            &["tcltest::debug ?level?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
