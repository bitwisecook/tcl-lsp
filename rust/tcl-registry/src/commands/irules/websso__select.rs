//! `WEBSSO::select` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WEBSSO::select",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Use specified SSO configuration object to do SSO for the HTTP request.",
            &["WEBSSO::select WEBSSO_OBJECT"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
