//! `PROFILE::access` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::access",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `PROFILE::access`.",
            &["PROFILE::access ATTR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
