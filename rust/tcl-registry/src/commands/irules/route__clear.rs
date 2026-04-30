//! `ROUTE::clear` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::clear",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Removes a Congestion Metrics Cache entry.",
            &["ROUTE::clear DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
