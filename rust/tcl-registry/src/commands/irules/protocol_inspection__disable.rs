//! `PROTOCOL_INSPECTION::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROTOCOL_INSPECTION::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disables inspection match of the flow.",
            synopsis: &["PROTOCOL_INSPECTION::disable"],
            snippet: "Disables inspection of the flow",
            source: "https://clouddocs.f5.com/api/irules/PROTOCOL_INSPECTION__disable.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "PROTOCOL_INSPECTION::disable",
        }],
        ..CommandSpec::DEFAULT
    }
}
