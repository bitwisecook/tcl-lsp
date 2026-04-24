//! `WS::payload_processing` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::payload_processing",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enables or disables processing of WebSocket payload via payload protocol filter",
            &["WS::payload_processing ('enable' | 'disable')"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
