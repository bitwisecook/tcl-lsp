//! `LB::prime` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::prime",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Prime server connections.",
            synopsis: &["LB::prime"],
            snippet: "Prime server connections",
            source: "https://clouddocs.f5.com/api/irules/LB__prime.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
