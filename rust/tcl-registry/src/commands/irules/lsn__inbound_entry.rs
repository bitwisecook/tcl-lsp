//! `LSN::inbound-entry` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::inbound-entry",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command creates and gets the inbound mapping for a translation address, tra",
            &["LSN::inbound-entry (get | delete) IP_TUPLE IP_PROTOCOL"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
