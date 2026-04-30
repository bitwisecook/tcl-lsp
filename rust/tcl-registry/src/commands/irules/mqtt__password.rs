//! `MQTT::password` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::password",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set password field of MQTT CONNECT message.",
            &["MQTT::password (PASSWORD)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
