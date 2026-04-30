//! `import_ip` command.
use crate::prelude::*;
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
        ..CommandSpec::DEFAULT
    }
}
