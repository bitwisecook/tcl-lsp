//! `read_verilog` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "read_verilog ?-define define_list? file_list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_verilog",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Read Verilog source files.",
            &["read_verilog ?-define define_list? file_list"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
