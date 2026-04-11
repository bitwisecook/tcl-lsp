//! `UDP::respond` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::respond",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sends data directly to a peer.",
            &["UDP::respond RESPONSE_STRING"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
