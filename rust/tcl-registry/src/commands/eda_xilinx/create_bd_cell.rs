//! `create_bd_cell` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "create_bd_cell -type ip -vlnv vlnv cell_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_bd_cell",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create a block design cell (IP instance).",
            &["create_bd_cell -type ip -vlnv vlnv cell_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
