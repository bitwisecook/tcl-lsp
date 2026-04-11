//! `MQTT::packet_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::packet_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set packet-id of MQTT message",
            &["MQTT::packet_id (PACKETID)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
