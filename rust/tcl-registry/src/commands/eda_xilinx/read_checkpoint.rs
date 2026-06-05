//! `read_checkpoint` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "read_checkpoint ?-cell cell? file_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_checkpoint",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Read a design checkpoint (.dcp).",
            &["read_checkpoint ?-cell cell? file_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
