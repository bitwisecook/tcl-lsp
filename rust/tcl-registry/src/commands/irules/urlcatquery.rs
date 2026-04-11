//! `urlcatquery` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "urlcatquery",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Query the URL for URL categorization.",
            &["urlcatquery URL_STRING"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
