//! `MQTT::disconnect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::disconnect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disconnect the MQTT connection.",
            &["MQTT::disconnect"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
