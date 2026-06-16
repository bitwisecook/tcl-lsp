//! `tcl::OptKeyRegister` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::OptKeyRegister",
        dialects: None,
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Register an option description list under a key for later use.",
            synopsis: &["tcl::OptKeyRegister optlist ?key?"],
            snippet: "",
            source: "Tcl stdlib opt package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("opt"),
        ..CommandSpec::DEFAULT
    }
}
