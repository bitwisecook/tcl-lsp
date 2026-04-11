//! `expect_before` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "expect_before",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Define patterns tested before each expect command.",
            &["expect_before ?-opts? pattern body ?pattern body ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
