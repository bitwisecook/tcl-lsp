//! `BWC::policy` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::policy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "The bwc irule allows a bwc policy to be attached or detached to a specific flow.",
            &["BWC::policy ('attach' | 'detach') POLICY_NAME (SESSION_ID)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
