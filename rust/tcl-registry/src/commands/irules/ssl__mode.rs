//! `SSL::mode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::mode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets the enabled/disabled state of SSL.",
            &["SSL::mode"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
