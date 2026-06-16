//! `tcl::OptProcArgGiven` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::OptProcArgGiven",
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return 1 if the named option was explicitly given, 0 otherwise.",
            synopsis: &["tcl::OptProcArgGiven name"],
            snippet: "",
            source: "Tcl stdlib opt package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("opt"),
        ..CommandSpec::DEFAULT
    }
}
