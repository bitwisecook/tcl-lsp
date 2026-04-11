//! `WS::request` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::request",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns the values of the various Websocket header fields seen in a",
            &["WS::request ('protocol' | 'extension' | 'version' | 'key' )"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
