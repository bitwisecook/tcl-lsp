//! `PROFILE::auth` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::auth",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of an authentication profile setting.",
            &["PROFILE::auth PROFILE_AUTH ATTR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
