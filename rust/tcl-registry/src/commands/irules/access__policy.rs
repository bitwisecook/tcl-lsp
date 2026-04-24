//! `ACCESS::policy` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::policy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Return information about access policies.",
            &["ACCESS::policy agent_id"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
