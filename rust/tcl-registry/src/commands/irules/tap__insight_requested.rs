//! `TAP::insight_requested` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TAP::insight_requested",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "TAP requests insight flag.",
            synopsis: &["TAP::insight_requested"],
            snippet: "Command for indication that TAP module wants to get insight from any module. TAP::insight must be invoked if flag is set to true.",
            source: "https://clouddocs.f5.com/api/irules/TAP__insight_requested.html",
            examples: "",
            return_value: "Return boolean value if TAP modules wants to get an insight from any module.",
        }),
        ..CommandSpec::DEFAULT
    }
}
