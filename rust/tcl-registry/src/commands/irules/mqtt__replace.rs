//! `MQTT::replace` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::replace",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Replace MQTT message",
            &["MQTT::replace ( (('type' 'CONNECT') ('client_id' NAME)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
