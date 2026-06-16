//! `tcl::OptKeyDelete` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::OptKeyDelete",
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Delete a previously registered option description.",
            synopsis: &["tcl::OptKeyDelete key"],
            snippet: "",
            source: "Tcl stdlib opt package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("opt"),
        ..CommandSpec::DEFAULT
    }
}
