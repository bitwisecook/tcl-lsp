//! `BOTDEFENSE::intent` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::intent",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the intent found for the bot that sent the current request.",
            &["BOTDEFENSE::intent"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
