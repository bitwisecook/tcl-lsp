//! `open_project` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "open_project ?-read_only? ?-quiet? project_file",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "open_project",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Open an existing Vivado project.",
            &["open_project ?-read_only? ?-quiet? project_file"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
