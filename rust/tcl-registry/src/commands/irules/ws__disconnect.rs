//! `WS::disconnect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::disconnect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command can be used to disconnect a Websocket connection.",
            &["WS::disconnect ( CODE (RSN)? )"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
