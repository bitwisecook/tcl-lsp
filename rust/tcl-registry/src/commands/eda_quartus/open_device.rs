//! `open_device` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "open_device",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Open a device on the JTAG chain.",
            &["open_device ?-hardware_name name? ?-device_name name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
