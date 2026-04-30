//! `HTTP::fallback` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::fallback",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Specifies or overrides a fallback host specified in the HTTP profile.",
            &["HTTP::fallback <host>"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
