//! `JSON::root` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::root",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets the JSON element (aka.",
            &["JSON::root (JSON_CACHE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
