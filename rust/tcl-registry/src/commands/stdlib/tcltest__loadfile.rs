//! `tcltest::loadFile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::loadFile",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary: "Get or set the load file path.  Deprecated: use ``configure -loadfile``.",
            synopsis: &["tcltest::loadFile ?filename?"],
            snippet: "",
            source: "Tcl stdlib tcltest package (deprecated)",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        deprecated_replacement: Some("tcltest::configure"),
        ..CommandSpec::DEFAULT
    }
}
