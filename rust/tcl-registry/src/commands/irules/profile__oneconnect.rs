//! `PROFILE::oneconnect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::oneconnect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of a Oneconnect profile setting.",
            &["PROFILE::oneconnect ATTR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
