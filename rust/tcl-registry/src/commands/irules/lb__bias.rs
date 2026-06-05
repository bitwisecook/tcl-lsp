//! `LB::bias` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::bias",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "F5 iRules command `LB::bias`.",
            synopsis: &["LB::bias (INTEGER)?"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/LB__bias.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "LB::bias (INTEGER)?",
        }],
        ..CommandSpec::DEFAULT
    }
}
