//! `AAA::auth_result` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AAA::auth_result",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command is used to check the result of an authentication request.",
            &["AAA::auth_result AAA_REQUEST_ID"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
