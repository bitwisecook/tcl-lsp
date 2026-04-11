//! `PROFILE::serverssl` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::serverssl",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of a Server SSL profile setting.",
            &["PROFILE::serverssl ATTR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
