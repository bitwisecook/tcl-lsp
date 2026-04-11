//! `radius_authenticate` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "radius_authenticate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "radius_authenticate command creates a RADIUS access request message, sends to th",
            &["radius_authenticate"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
