//! `htmlencode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "htmlencode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "HTML-encode a string (alias for HTML::encode).",
            &["htmlencode STRING"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
