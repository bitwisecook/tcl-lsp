//! `PSM::SMTP::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSM::SMTP::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "To enable PSM for SMTP traffic.",
            synopsis: &["PSM::SMTP::enable"],
            snippet: "To enable PSM for SMTP traffic",
            source: "https://clouddocs.f5.com/api/irules/PSM__SMTP__enable.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
