//! `MQTT::retain` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::retain",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set retain flag of MQTT PUBLISH message.",
            &["MQTT::retain ('0' | '1')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
