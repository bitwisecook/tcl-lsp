//! `IKE::san_ipadd` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IKE::san_ipadd",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "something",
            synopsis: &["IKE::san_ipadd (ANY_CHARS)*"],
            snippet: "something",
            source: "https://clouddocs.f5.com/api/irules/IKE__san_ipadd.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
