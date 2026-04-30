//! `vlog` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "vlog",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Compile Verilog/SystemVerilog source files.", &["vlog ?-work library? ?-sv? ?+define+name=val? ?+incdir+dir? ?-lint? ?-suppress n? ?-nowarn n? file_l"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
