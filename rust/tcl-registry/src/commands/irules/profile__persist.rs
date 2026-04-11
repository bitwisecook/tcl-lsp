//! `PROFILE::persist` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::persist",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of a persistence profile setting.",
            &["PROFILE::persist ((instance PROFILE_PERSIST ATTR) | (mode MODE ATTR))"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
