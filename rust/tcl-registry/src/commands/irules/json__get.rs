//! `JSON::get` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::get",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets the value content of a JSON element.",
            &["JSON::get JSON_ELEMENT (JSON_TYPE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
