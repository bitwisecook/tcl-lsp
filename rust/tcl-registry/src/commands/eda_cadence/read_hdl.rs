//! `read_hdl` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "read_hdl ?-language language? ?-define define_list? file_list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_hdl",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Read HDL source files (Verilog/SystemVerilog/VHDL).",
            &["read_hdl ?-language language? ?-define define_list? file_list"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
