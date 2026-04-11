//! `MQTT::username` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::username",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set user-name field of MQTT CONNECT message.",
            &["MQTT::username (NAME)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
