//! `tcl::tm::roots` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::tm::roots",
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Set the root paths for Tcl module discovery.",
            synopsis: &["tcl::tm::roots pathList"],
            snippet: "Given a list of root paths, computes version-specific sub-directories and adds them to the module search list.",
            source: "Tcl stdlib tm module system",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
