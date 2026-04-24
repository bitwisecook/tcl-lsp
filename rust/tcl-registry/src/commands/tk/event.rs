//! `event` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "event",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Generate, manage, and inspect virtual events.",
            &["event add <<virtual>> sequence ?sequence ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
