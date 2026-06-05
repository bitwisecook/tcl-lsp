//! `PSM::SMTP::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSM::SMTP::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "To disable PSM for SMTP traffic.",
            synopsis: &["PSM::SMTP::disable"],
            snippet: "To disable PSM for SMTP traffic",
            source: "https://clouddocs.f5.com/api/irules/PSM__SMTP__disable.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
