//! `connect_hw_server` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "connect_hw_server",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Connect to a hardware server.",
            &["connect_hw_server ?-url url? ?-allow_non_jtag?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
