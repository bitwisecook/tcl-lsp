//! `ACCESS2::access2_proc` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS2::access2_proc",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command is used to get the TCL procedure registered for currently executing per-request policy expression.",
            synopsis: &["ACCESS2::access2_proc"],
            snippet: "This command will return the TCL procedure registered for currently executing per-request policy expression.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS2__access2_proc.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
