//! `MQTT::qos` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::qos",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set qos of MQTT PUBLISH message",
            &["MQTT::qos ('0' | '1' | '2')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
