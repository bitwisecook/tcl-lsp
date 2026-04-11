//! `ACCESS::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enables the access control enforcement for a particular request URI.",
            &["ACCESS::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
