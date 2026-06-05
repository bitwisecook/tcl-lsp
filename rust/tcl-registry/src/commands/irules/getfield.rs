//! `getfield` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "getfield",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Splits a string on a character or string.",
            synopsis: &["getfield STRING SEPARATOR FIELD_NUMBER"],
            snippet: "A custom iRule function which splits a string on a character or\nstring, and returns the string corresponding to the specific field.",
            source: "https://clouddocs.f5.com/api/irules/getfield.html",
            examples: "when HTTP_REQUEST {\n  set hostname [getfield [HTTP::host] \":\" 1]\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
