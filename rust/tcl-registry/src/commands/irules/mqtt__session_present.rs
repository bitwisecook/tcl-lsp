//! `MQTT::session_present` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::session_present",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set session_present flag of MQTT CONNACK message.",
            &["MQTT::session_present ('0' | '1')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
