//! `WEBSSO::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WEBSSO::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Causes APM to do the SSO processing on a request.",
            &["WEBSSO::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
