//! `JSON::render` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::render",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns a string containing a textual rendering of the JSON cache content.",
            &["JSON::render (JSON_CACHE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
