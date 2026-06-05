//! `b64decode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "b64decode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns a string that is base-64 decoded.",
            synopsis: &["b64decode ANY_CHARS"],
            snippet: "Returns a string that is base-64 decoded.",
            source: "https://clouddocs.f5.com/api/irules/b64decode.html",
            examples: "when RULE_INIT {\n   set ::key [AES::key]\n}",
            return_value: "b64decode <string> Returns a string that is base-64 decoded",
        }),
        ..CommandSpec::DEFAULT
    }
}
