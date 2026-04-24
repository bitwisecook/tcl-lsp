//! `HTTP::redirect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::redirect",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Redirects an HTTP request or response to the specified URL.",
            &["HTTP::redirect REDIRECT_URL"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
