//! `AM::policy_node` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AM::policy_node",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `AM::policy_node`.",
            &["AM::policy_node"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
