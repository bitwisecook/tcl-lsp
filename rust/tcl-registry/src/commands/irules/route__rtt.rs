//! `ROUTE::rtt` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::rtt",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the cached round-trip-time estimate.",
            &["ROUTE::rtt DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
