//! `create_bd_port` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_bd_port",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create a block design external port.",
            &["create_bd_port -dir direction ?-type type? port_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
