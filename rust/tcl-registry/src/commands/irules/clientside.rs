//! `clientside` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "clientside",
        traits: Traits::IS_SIDE_SWITCH,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Causes the specified iRule commands to be evaluated under the client-side contex",
            &["clientside (NESTING_SCRIPT)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
