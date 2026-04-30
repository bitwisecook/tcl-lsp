//! `HTTP::proxy` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::proxy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Controls the application of HTTP proxy when using an Explicit HTTP profile.",
            &["HTTP::proxy"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
