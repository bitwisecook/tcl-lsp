//! `MQTT::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Manipulate payload of MQTT PUBLISH message",
            &["MQTT::payload ?subcommand? ?args?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
