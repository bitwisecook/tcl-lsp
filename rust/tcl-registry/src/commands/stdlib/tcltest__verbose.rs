//! `tcltest::verbose` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::verbose",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary: "Get or set verbosity level.  Deprecated: use ``configure -verbose``.",
            synopsis: &["tcltest::verbose ?level?"],
            snippet: "",
            source: "Tcl stdlib tcltest package (deprecated)",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        ..CommandSpec::DEFAULT
    }
}
