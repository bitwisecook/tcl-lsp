//! `HTTP::hsts` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::hsts",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 2),
        hover: Some(HoverSnippet::brief(
            "Controls HTTP Strict Transport Security.",
            &["HTTP::hsts"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
