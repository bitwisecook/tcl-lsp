//! `JSON::array` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::array",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "A group of subcommands that operate on a JSON array.",
            &["JSON::array ("],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
