//! `WS::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Queries for or manipulates Websocket frame payload information.",
            &["WS::payload (LENGTH | (OFFSET LENGTH))?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
