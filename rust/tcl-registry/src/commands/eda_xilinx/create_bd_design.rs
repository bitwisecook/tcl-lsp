//! `create_bd_design` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "create_bd_design ?-dir dir? design_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_bd_design",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Create a block design.",
            &["create_bd_design ?-dir dir? design_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
