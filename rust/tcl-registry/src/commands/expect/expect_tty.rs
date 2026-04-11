//! `expect_tty` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "expect_tty",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Expect input from the controlling terminal (tty).",
            &["expect_tty ?-opts? pattern body ?pattern body ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
