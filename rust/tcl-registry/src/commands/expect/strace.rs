//! `strace` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "strace",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Trace Expect internal statements at the given detail level.",
            &["strace level"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
