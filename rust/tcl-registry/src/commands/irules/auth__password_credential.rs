//! `AUTH::password_credential` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::password_credential",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the password credential to the specified string for a future AUTH::authenti",
            &["AUTH::password_credential AUTH_ID PASSWORD_CREDENTIAL"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
