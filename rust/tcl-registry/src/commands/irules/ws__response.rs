//! `WS::response` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::response",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns the values of the various Websocket header fields seen in a",
            &["WS::response ('protocol' | 'extension' | 'version' | 'key' | 'valid' )"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
