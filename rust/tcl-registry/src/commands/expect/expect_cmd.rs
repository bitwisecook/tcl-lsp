//! `expect` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "expect",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Wait for output matching a pattern from a spawned process.",
            &["expect ?-opts? pattern body ?pattern body ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
