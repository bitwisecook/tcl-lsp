//! `BOTDEFENSE::micro_service` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::micro_service",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the micro-service that matched the current request.",
            &["BOTDEFENSE::micro_service (name | type)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
