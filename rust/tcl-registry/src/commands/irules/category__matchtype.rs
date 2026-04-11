//! `CATEGORY::matchtype` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CATEGORY::matchtype",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get the type of match found.",
            &["CATEGORY::matchtype TYPE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
