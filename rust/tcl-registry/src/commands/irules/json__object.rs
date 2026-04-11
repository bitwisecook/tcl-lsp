//! `JSON::object` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::object",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "A group of subcommands that operate on a JSON object.",
            &["JSON::object ("],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
