//! `tcltest::loadFile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::loadFile",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set the load file path.  Deprecated: use ``configure -loadfile``.",
            &["tcltest::loadFile ?filename?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
