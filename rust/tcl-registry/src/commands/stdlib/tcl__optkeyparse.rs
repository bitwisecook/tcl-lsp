//! `tcl::OptKeyParse` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::OptKeyParse",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Parse arguments using a previously registered option description.",
            synopsis: &["tcl::OptKeyParse key arglist"],
            snippet: "",
            source: "Tcl stdlib opt package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("opt"),
        ..CommandSpec::DEFAULT
    }
}
