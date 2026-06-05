//! `tcltest::preserveCore` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::preserveCore",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary:
                "Get or set core file preservation.  Deprecated: use ``configure -preservecore``.",
            synopsis: &["tcltest::preserveCore ?level?"],
            snippet: "",
            source: "Tcl stdlib tcltest package (deprecated)",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        ..CommandSpec::DEFAULT
    }
}
