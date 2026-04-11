//! `http_host` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http_host",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of the HTTP Host header.",
            &["http_host"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
