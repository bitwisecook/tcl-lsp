//! `HTTP2::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Changes the HTTP2 filter from full parsing to passthrough mode.",
            &["HTTP2::disable ('clientside')? ('serverside')? ('discard')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
