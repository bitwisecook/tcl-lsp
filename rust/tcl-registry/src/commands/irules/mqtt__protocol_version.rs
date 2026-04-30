//! `MQTT::protocol_version` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::protocol_version",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set protocol revision level of MQTT CONNECT message",
            &["MQTT::protocol_version (VERSION)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
