//! `BOTDEFENSE::cookie_age` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::cookie_age",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the age of the Bot Defense cookie in seconds.",
            &["BOTDEFENSE::cookie_age"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
