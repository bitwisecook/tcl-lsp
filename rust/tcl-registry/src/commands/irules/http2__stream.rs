//! `HTTP2::stream` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::stream",
        traits: Traits::PURE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 2),
        hover: Some(HoverSnippet::brief(
            "Gets or sets the stream attributes including id and priority.",
            &["HTTP2::stream (id | (priority (PRIORITY)?))?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
