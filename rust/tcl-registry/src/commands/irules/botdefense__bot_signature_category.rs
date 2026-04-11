//! `BOTDEFENSE::bot_signature_category` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::bot_signature_category",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the name of the detected Bot Signature Category.",
            &["BOTDEFENSE::bot_signature_category"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
