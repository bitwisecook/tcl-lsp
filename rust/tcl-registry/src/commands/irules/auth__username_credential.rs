//! `AUTH::username_credential` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::username_credential",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the username credential to a string.",
            &["AUTH::username_credential AUTH_ID USERNAME_CREDENTIAL"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
