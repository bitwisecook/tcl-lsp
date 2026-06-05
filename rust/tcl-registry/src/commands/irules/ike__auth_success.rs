//! `IKE::auth_success` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IKE::auth_success",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "something",
            synopsis: &["IKE::auth_success (ANY_CHARS)*"],
            snippet: "something",
            source: "https://clouddocs.f5.com/api/irules/IKE__auth_success.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
