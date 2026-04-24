//! `system` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "system",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Execute a command string via the system shell (/bin/sh -c).",
            &["system args"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
