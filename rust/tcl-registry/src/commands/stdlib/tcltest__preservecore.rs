//! `tcltest::preserveCore` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::preserveCore",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set core file preservation.  Deprecated: use ``configure -preservecore``.",
            &["tcltest::preserveCore ?level?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
