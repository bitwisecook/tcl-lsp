//! `event` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "event",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enables or disables evaluation of the specified iRule event or all iRule events ",
            &["event info"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
