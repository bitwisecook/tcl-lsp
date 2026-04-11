//! `MQTT::release` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::release",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Releases the data collected via MQTT::collect iRule command",
            &["MQTT::release"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
