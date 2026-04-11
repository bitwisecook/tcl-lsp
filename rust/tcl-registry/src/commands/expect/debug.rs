//! `debug` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "debug",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enable or disable the Expect debugger.",
            &["debug ?-now? ?0 | 1?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
