//! `MQTT::insert` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::insert",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Insert an MQTT message",
            &["MQTT::insert (('before' | 'after') ("],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
