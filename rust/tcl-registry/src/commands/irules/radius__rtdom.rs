//! `RADIUS::rtdom` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RADIUS::rtdom",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command overwrites the default route-domain ID in RADIUS scenario with give",
            &["RADIUS::rtdom (ROUTE_DOMAIN)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
