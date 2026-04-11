//! `MQTT::keep_alive` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::keep_alive",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set keep_alive field of MQTT CONNECT message.",
            &["MQTT::keep_alive (KEEP_ALIVE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
