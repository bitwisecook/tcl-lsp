//! `close_device` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "close_device",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Close the active JTAG device.",
            &["close_device"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
