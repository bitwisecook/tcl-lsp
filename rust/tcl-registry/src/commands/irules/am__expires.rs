//! `AM::expires` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AM::expires",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "F5 iRules command `AM::expires`.",
            synopsis: &["AM::expires"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/AM__expires.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
