//! `MQTT::return_code` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::return_code",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set return-code field of MQTT CONNACK message.",
            &["MQTT::return_code (RETURN_CODE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
