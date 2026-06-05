//! `read_edif` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "read_edif file_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_edif",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Read an EDIF netlist file.",
            &["read_edif file_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
