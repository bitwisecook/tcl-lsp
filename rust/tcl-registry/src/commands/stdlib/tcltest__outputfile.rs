//! `tcltest::outputFile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::outputFile",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set the output file.  Deprecated: use ``configure -outfile``.",
            &["tcltest::outputFile ?filename?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
