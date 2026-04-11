//! `HTML::comment` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTML::comment",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Query and update HTML comment.",
            &["HTML::comment ((append STRING) | (prepend STRING) | remove)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
