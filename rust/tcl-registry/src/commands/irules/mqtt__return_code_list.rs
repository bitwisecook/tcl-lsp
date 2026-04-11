//! `MQTT::return_code_list` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::return_code_list",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get return-code-list of MQTT SUBACK message.",
            &["MQTT::return_code_list"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
