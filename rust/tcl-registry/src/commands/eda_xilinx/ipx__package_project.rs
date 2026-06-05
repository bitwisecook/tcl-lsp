//! `ipx::package_project` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ipx::package_project ?-root_dir dir? ?-import_files? ?-force?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ipx::package_project",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Package a project as an IP core.",
            &["ipx::package_project ?-root_dir dir? ?-import_files? ?-force?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
