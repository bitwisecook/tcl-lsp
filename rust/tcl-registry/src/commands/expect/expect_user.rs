//! `expect_user` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "expect_user",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Expect input from the user (standard input).",
            &["expect_user ?-opts? pattern body ?pattern body ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
