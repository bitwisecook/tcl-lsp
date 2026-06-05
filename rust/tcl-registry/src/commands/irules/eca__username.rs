//! `ECA::username` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ECA::username",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns NTLM authenticating username.",
            synopsis: &["ECA::username"],
            snippet: "The ECA::username command returns NTLM authenticating username.",
            source: "https://clouddocs.f5.com/api/irules/ECA__username.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
