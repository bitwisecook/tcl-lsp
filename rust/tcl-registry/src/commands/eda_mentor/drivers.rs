//! `drivers` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "drivers",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Find drivers of a signal.",
            &["drivers signal_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
