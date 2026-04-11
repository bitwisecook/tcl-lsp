//! `stty` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "stty",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set or query terminal modes (raw, echo, rows, columns, etc.).",
            &["stty ?args?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
