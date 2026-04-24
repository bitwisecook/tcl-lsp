//! `HTTP2::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Changes the HTTP2 filter from passthrough to full parsing mode.",
            &["HTTP2::enable ('clientside')? ('serverside')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
