//! `URI::compare` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::compare",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Compares two URI's for equality.",
            &["URI::compare URI_STRING URI_STRING"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
