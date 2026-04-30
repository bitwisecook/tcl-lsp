//! `MQTT::protocol_name` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::protocol_name",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set protocol-name of MQTT CONNECT message",
            &["MQTT::protocol_name (PROTOCOL)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
