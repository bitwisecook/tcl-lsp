//! `AM::application` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AM::application",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `AM::application`.",
            &["AM::application"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
