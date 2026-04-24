//! `IKE::auth_success` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IKE::auth_success",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "something",
            &["IKE::auth_success (ANY_CHARS)*"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
