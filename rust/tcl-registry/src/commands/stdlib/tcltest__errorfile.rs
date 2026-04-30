//! `tcltest::errorFile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::errorFile",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set the error output file.  Deprecated: use ``configure -errfile``.",
            &["tcltest::errorFile ?filename?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
