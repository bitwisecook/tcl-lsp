//! `HTTP2::version` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::version",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command can be used to determine the HTTP/2 protocol version used.",
            &["HTTP2::version"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
