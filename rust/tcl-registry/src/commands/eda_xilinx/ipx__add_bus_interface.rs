//! `ipx::add_bus_interface` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ipx::add_bus_interface",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Add a bus interface to a packaged IP.", &["ipx::add_bus_interface -abstraction_type_vlnv vlnv -bus_type_vlnv vlnv interface_name component"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
