//! `create_ip` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_ip",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Create an IP core instance.", &["create_ip -name name -vendor vendor -library library -version version ?-module_name mod_name? ?-dir dir?"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
