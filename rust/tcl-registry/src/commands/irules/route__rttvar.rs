//! `ROUTE::rttvar` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::rttvar",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the cached round-trip-time variance (rttvar) estimate.",
            &["ROUTE::rttvar DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
