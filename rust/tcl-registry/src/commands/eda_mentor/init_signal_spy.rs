//! `init_signal_spy` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "init_signal_spy",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Initialize signal spy for cross-hierarchy access.",
            &["init_signal_spy src_signal dst_signal ?-node? ?-verbose?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
