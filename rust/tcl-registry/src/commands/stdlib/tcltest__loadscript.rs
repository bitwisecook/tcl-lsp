//! `tcltest::loadScript` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::loadScript",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set the load script.  Deprecated: use ``configure -load``.",
            &["tcltest::loadScript ?script?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
