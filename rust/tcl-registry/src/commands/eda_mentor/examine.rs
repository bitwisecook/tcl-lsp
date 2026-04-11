//! `examine` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "examine",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Examine the value of a signal or variable.",
            &["examine ?-radix radix? ?-time time? signal_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
