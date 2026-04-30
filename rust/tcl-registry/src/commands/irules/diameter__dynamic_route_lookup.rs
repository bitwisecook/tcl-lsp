//! `DIAMETER::dynamic_route_lookup` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::dynamic_route_lookup",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set whether messages should be routed dynamically.",
            &["DIAMETER::dynamic_route_lookup ( connection | message ) ( BOOLEAN )?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
