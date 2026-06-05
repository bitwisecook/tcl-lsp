//! `read_vhdl` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "read_vhdl ?-library lib? ?-vhdl2008? file_list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_vhdl",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Add VHDL source files to the project.",
            &["read_vhdl ?-library lib? ?-vhdl2008? file_list"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
