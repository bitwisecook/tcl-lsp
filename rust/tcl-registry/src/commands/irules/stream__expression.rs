//! `STREAM::expression` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "STREAM::expression",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Replaces the expression in a Stream profile with another expression.",
            &["STREAM::expression EXPRESSION"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
