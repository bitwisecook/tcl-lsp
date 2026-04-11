//! `SIP::route_status` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::route_status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the routing status of the current message.",
            &["SIP::route_status"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
