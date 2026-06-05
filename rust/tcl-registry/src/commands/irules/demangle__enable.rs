//! `DEMANGLE::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DEMANGLE::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "F5 iRules command `DEMANGLE::enable`.",
            synopsis: &["DEMANGLE::enable"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/DEMANGLE__enable.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
