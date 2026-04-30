//! `send` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "send",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Send a string to the current spawned process.",
            &["send ?-flags? string"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
