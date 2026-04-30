//! `CATEGORY::safesearch` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CATEGORY::safesearch",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get safe search key and value pairs.",
            &["CATEGORY::safesearch URL ('-ip' IP)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
