//! `HTTP2::disconnect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::disconnect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command allows you to cleanly terminate the current HTTP/2 session.",
            &["HTTP2::disconnect"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
