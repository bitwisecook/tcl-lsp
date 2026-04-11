//! `MQTT::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disable MQTT parsing on a connection.",
            &["MQTT::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
