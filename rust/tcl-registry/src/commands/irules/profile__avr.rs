//! `PROFILE::avr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::avr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the value of a avr profile setting.",
            synopsis: &["PROFILE::avr ATTR"],
            snippet:
                "Returns the current value of the specified setting in the assigned avr profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__avr.html",
            examples: "",
            return_value:
                "Returns the current value of the specified setting in the assigned avr profile.",
        }),
        ..CommandSpec::DEFAULT
    }
}
