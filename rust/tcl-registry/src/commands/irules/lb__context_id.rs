//! `LB::context_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::context_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Assign the current connection to a named context.",
            synopsis: &["LB::context_id"],
            snippet: "Assign the current connection to a named context.",
            source: "https://clouddocs.f5.com/api/irules/LB__context_id.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "LB::context_id",
        }],
        ..CommandSpec::DEFAULT
    }
}
