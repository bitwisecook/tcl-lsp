//! `IKE::san_email` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IKE::san_email",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "something",
            synopsis: &["IKE::san_email (ANY_CHARS)*"],
            snippet: "something",
            source: "https://clouddocs.f5.com/api/irules/IKE__san_email.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
