//! `ROUTE::cwnd` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::cwnd",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the cached congestion window (cwnd) value.",
            &["ROUTE::cwnd DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
