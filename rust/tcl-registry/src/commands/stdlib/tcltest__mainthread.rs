//! `tcltest::mainThread` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::mainThread",
        dialects: None,
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary: "Get or set the main thread ID.",
            synopsis: &["tcltest::mainThread ?id?"],
            snippet: "",
            source: "Tcl stdlib tcltest package (v1 compat)",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        ..CommandSpec::DEFAULT
    }
}
