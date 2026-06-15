//! `tcltest::matchDirectories` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::matchDirectories",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary: "Get or set matching directory patterns.  Deprecated: use ``configure -relateddir``.",
            synopsis: &["tcltest::matchDirectories ?patternList?"],
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
