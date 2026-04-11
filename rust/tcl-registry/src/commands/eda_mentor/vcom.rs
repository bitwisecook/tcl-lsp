//! `vcom` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "vcom",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Compile VHDL source files.", &["vcom ?-work library? ?-2008? ?-explicit? ?-check_synthesis? ?-lint? ?-suppress n? ?-nowarn n? file_l"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
