//! `MQTT::message` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::message",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the full content of the MQTT message.",
            &["MQTT::message"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
