//! `HTTP::is_keepalive` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::is_keepalive",
        traits: Traits::PURE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Returns a true value if this is a Keep-Alive connection.",
            &["HTTP::is_keepalive"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
