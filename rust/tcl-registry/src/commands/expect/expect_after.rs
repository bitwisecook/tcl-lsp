//! `expect_after` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "expect_after",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Define patterns tested after each expect command.",
            &["expect_after ?-opts? pattern body ?pattern body ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
