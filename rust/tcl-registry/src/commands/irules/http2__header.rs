//! `HTTP2::header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet::brief(
            "Queries or modifies HTTP/2 pseudo-headers.",
            &["HTTP2::header <name>"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
