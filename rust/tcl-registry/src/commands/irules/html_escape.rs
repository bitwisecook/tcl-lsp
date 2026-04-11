//! `html_escape` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "html_escape",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "HTML-escape a string (alias for HTML::encode).",
            &["html_escape STRING"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
