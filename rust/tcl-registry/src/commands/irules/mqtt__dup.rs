//! `MQTT::dup` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::dup",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set duplicate flag of MQTT PUBLISH message.",
            &["MQTT::dup ('0' | '1')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
