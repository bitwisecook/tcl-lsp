//! `LB::command` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::command",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "LB::command",
            synopsis: &["LB::command ('transparent_port')?"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/lb__command.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
