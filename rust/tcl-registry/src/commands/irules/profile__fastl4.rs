//! `PROFILE::fastL4` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::fastL4",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of a Fast L4 profile setting.",
            &["PROFILE::fastL4 ATTR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
