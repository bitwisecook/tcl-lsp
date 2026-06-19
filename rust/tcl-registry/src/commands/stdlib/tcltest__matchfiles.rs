//! `tcltest::matchFiles` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::matchFiles",
        dialects: None,
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary: "Get or set matching file patterns.  Deprecated: use ``configure -file``.",
            synopsis: &["tcltest::matchFiles ?patternList?"],
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
