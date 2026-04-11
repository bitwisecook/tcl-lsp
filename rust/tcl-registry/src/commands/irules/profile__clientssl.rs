//! `PROFILE::clientssl` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::clientssl",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of a Client SSL profile setting.",
            &["PROFILE::clientssl ATTR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
