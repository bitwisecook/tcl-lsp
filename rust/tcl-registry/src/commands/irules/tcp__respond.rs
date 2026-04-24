//! `TCP::respond` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::respond",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sends the specified data directly to the peer.",
            &["TCP::respond RESPONSE_STRING"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
