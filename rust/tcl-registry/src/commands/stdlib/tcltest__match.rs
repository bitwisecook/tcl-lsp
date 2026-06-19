//! `tcltest::match` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::match",
        dialects: None,
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary: "Get or set test match patterns.  Deprecated: use ``configure -match``.",
            synopsis: &["tcltest::match ?patternList?"],
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
