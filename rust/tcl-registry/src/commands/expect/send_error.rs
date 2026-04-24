//! `send_error` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "send_error",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Send a string to standard error.",
            &["send_error ?-flags? string"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
