//! `WS::message` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::message",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command can be used to drop an entire Websocket message.",
            &["WS::message ( 'drop' )"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
