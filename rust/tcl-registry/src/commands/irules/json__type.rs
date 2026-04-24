//! `JSON::type` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets the type of the given JSON element (aka.",
            &["JSON::type JSON_ELEMENT"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
