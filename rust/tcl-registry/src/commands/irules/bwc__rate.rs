//! `BWC::rate` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::rate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command is used to modify max-user rate for dynamic policy.",
            &["BWC::rate SESSION_ID BW_VALUE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
