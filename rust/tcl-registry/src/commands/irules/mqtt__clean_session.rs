//! `MQTT::clean_session` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::clean_session",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set clean_session flag of MQTT CONNECT message.",
            &["MQTT::clean_session ('0' | '1')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
