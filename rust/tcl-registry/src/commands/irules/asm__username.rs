//! `ASM::username` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::username",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "request username from a login attempt throughout the login session.",
            &["ASM::username"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
