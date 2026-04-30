//! `init_signal_driver` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "init_signal_driver",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Initialize signal driver for cross-hierarchy driving.",
            &["init_signal_driver src_signal dst_signal ?-default default_value? ?-delay delay?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
