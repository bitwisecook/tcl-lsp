//! `log_user` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "log_user",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Control whether send/expect output is logged to stdout.",
            &["log_user -info"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
