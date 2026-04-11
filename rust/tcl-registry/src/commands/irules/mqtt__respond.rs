//! `MQTT::respond` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::respond",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Transmit MQTT message to sender",
            &["MQTT::respond ( (('type' 'CONNACK') ('return_code' RETURN_CODE)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
