//! `NTLM::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "NTLM::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enables processing for NTLM.",
            synopsis: &["NTLM::enable"],
            snippet: "Enables processing for NTLM",
            source: "https://clouddocs.f5.com/api/irules/NTLM__enable.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
