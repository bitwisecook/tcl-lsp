//! `PSC::imeisv` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::imeisv",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get or set imeisv value.",
            synopsis: &["PSC::imeisv (IMEISV)?"],
            snippet: "The PSC::imeisv command gets the imeisv or sets the imeisv when the\noptional value is given.",
            source: "https://clouddocs.f5.com/api/irules/PSC__imeisv.html",
            examples: "",
            return_value: "Return the imeisv value when no argument is given.",
        }),
        ..CommandSpec::DEFAULT
    }
}
