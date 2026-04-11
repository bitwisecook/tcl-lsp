//! `MQTT::collect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::collect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Collect the specified amount of MQTT message payload data",
            &["MQTT::collect (COLLECT)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
