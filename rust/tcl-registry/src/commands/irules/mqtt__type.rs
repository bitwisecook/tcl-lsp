//! `MQTT::type` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get type of MQTT message",
            &["MQTT::type"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
