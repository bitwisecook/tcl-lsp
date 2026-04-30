//! `BOTDEFENSE::bot_signature` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::bot_signature",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the name of the detected Bot Signature.",
            &["BOTDEFENSE::bot_signature"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
