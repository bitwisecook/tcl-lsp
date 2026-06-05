//! `b64encode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "b64encode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns a string that is base-64 encoded, or if an error occurs, an empty string.",
            synopsis: &["b64encode ANY_CHARS"],
            snippet: "Returns a string that is base-64 encoded, or if an error occurs, an empty string.",
            source: "https://clouddocs.f5.com/api/irules/b64encode.html",
            examples: "when RULE_INIT {\n    set ::key [AES::key]\n}",
            return_value: "b64encode <string> Returns a string that is base-64 encoded, or if an error occurs, an empty string.",
        }),
        ..CommandSpec::DEFAULT
    }
}
