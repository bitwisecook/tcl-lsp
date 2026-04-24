//! `redirect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "redirect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Redirects an HTTP request to the specific location.",
            &["redirect to HOST_URI"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
