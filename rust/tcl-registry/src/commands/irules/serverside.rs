//! `serverside` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "serverside",
        traits: Traits::IS_SIDE_SWITCH,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Causes the specified iRule command to be evaluated under the server-side context",
            &["serverside (NESTING_SCRIPT)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
