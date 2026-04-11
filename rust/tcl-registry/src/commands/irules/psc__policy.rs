//! `PSC::policy` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::policy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get/set/remove policies.",
            &["PSC::policy (POLICY_NAME)*"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
