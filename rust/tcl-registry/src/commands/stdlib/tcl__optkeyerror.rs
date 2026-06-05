//! `tcl::OptKeyError` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::OptKeyError",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Generate an error message for a registered option description.",
            synopsis: &["tcl::OptKeyError key ?prefix?"],
            snippet: "",
            source: "Tcl stdlib opt package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("opt"),
        ..CommandSpec::DEFAULT
    }
}
