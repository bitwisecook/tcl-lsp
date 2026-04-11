//! `tcltest::outputChannel` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::outputChannel",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set the channel for test output.",
            &["tcltest::outputChannel ?channelID?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
