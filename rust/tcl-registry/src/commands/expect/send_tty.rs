//! `send_tty` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "send_tty",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Send a string to the controlling terminal (tty).",
            &["send_tty ?-flags? string"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
