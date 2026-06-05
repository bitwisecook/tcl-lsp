//! `tcltest::skipFiles` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::skipFiles",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary: "Get or set file skip patterns.  Deprecated: use ``configure -notfile``.",
            synopsis: &["tcltest::skipFiles ?patternList?"],
            snippet: "",
            source: "Tcl stdlib tcltest package (deprecated)",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        ..CommandSpec::DEFAULT
    }
}
