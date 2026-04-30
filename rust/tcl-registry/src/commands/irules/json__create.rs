//! `JSON::create` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::create",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Creates a new, empty JSON cache instance.",
            &["JSON::create"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
