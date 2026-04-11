//! `HTML::encode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTML::encode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "HTML-encodes a string, escaping special characters.",
            &["HTML::encode STRING"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
