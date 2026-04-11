//! `MQTT::topic` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::topic",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Manipulate topic(s) of MQTT message.",
            &["MQTT::topic ?subcommand? ?args?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
