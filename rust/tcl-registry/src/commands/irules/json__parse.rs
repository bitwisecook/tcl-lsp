//! `JSON::parse` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::parse",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Parses JSON content into a JSON cache that can be manipulated using further JSON",
            &["JSON::parse (JSON_STRING (JSON_MAX_ENTRIES)? )?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
