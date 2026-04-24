//! `HTTP::is_redirect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::is_redirect",
        traits: Traits::PURE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Returns a true value if the response is a redirect.",
            &["HTTP::is_redirect"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
