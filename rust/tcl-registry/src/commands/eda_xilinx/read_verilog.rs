//! `read_verilog` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "read_verilog ?-sv? ?-library lib? file_list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_verilog",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Add Verilog source files to the project.",
            &["read_verilog ?-sv? ?-library lib? file_list"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
