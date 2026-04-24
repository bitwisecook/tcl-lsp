//! `read_xdc` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_xdc",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Read XDC timing constraint files.",
            &["read_xdc ?-unmanaged? ?-ref ref_name? ?-cells cell? file_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
