//! `connect_bd_net` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "connect_bd_net",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Connect block design nets.",
            &["connect_bd_net ?-net net_name? pin_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
