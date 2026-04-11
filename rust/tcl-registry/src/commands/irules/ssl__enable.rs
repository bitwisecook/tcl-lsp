//! `SSL::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::enable",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Re-enables SSL processing.",
            &["SSL::enable (clientside | serverside)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
