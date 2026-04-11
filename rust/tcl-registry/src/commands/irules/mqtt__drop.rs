//! `MQTT::drop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::drop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Drop the current MQTT message.",
            &["MQTT::drop"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
