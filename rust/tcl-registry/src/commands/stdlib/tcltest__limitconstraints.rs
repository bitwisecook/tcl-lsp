//! `tcltest::limitConstraints` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::limitConstraints",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary: "Get or set constraint limiting.  Deprecated: use ``configure -limitconstraints``.",
            synopsis: &["tcltest::limitConstraints ?boolean?"],
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
