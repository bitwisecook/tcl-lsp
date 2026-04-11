//! `set_wire_load_mode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_wire_load_mode",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Set the wire load mode.",
            &["set_wire_load_mode mode"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
