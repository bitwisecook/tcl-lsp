//! `getfield` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "getfield",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Splits a string on a character or string.",
            &["getfield STRING SEPARATOR FIELD_NUMBER"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
