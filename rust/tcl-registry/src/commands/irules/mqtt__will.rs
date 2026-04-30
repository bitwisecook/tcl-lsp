//! `MQTT::will` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::will",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set will-topic, will-message, will-qos, and will-retain fields of MQTT CO",
            &["MQTT::will (('topic' (TOPIC)?) |"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
