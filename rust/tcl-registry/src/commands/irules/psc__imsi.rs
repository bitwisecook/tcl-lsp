//! `PSC::imsi` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::imsi",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set the imsi value.",
            synopsis: &["PSC::imsi (IMSI)?"],
            snippet:
                "The PSC::imsi command gets the imsi or sets the imsi when the optional\nis given.",
            source: "https://clouddocs.f5.com/api/irules/PSC__imsi.html",
            examples: "",
            return_value: "Return the imsi value when no argument is given.",
        }),
        ..CommandSpec::DEFAULT
    }
}
