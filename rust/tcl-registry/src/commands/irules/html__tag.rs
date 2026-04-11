//! `HTML::tag` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTML::tag",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Query and update the HTML tag.",
            &["HTML::tag ((append STRING) | name | (prepend STRING) | remove)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
