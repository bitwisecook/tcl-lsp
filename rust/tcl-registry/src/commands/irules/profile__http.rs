//! `PROFILE::http` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::http",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of an HTTP profile setting.",
            &["PROFILE::http ATTR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
