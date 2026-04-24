//! `ROUTE::expiration` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::expiration",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the remaining time for a route or congestion metrics cache entry.",
            &["ROUTE::expiration DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
