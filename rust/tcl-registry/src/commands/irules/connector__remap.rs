//! `CONNECTOR::remap` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CONNECTOR::remap",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set client/server IP/Port from connector.",
            &["CONNECTOR::remap server_addr IP_ADDR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
