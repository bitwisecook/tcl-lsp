//! `exp_internal` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exp_internal",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Control Expect internal diagnostic output.",
            &["exp_internal ?-f file? 0|1"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
