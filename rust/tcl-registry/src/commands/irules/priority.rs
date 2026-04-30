//! `priority` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "priority",
        traits: Traits::IRULES_TOP_LEVEL_ONLY,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the order of execution for iRule events.",
            &["priority EVENT_PRIORITY"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
