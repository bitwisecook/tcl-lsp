//! `tcltest::errorChannel` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::errorChannel",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set the channel for test error output.",
            &["tcltest::errorChannel ?channelID?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
