//! `import_ip` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "import_ip ?-srcset srcset? file_list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "import_ip",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Import an IP core into the project.",
            &["import_ip ?-srcset srcset? file_list"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
