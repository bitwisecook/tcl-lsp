//! `send_log` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "send_log",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Send a string to the log file only (not to the process or user).",
            &["send_log ?--? string"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
