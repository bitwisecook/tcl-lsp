//! `html_encode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "html_encode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "HTML-encode a string (alias for HTML::encode).",
            &["html_encode STRING"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
