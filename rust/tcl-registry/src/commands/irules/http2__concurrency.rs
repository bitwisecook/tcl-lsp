//! `HTTP2::concurrency` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::concurrency",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command can be used to determine the number of active concurrent streams in",
            &["HTTP2::concurrency"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
