//! `MQTT::length` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::length",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get length of MQTT message",
            &["MQTT::length"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
