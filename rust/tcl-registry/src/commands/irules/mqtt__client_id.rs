//! `MQTT::client_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::client_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set client identifier of MQTT CONNECT message",
            &["MQTT::client_id (CLIENTID)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
