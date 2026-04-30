//! `HTTP2::requests` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::requests",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command can be used to determine the count of requests received in the curr",
            &["HTTP2::requests"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
