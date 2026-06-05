//! `AM::policy_node` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AM::policy_node",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "F5 iRules command `AM::policy_node`.",
            synopsis: &["AM::policy_node"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/AM__policy_node.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
