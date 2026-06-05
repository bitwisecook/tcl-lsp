//! `read_ip` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "read_ip ?-quiet? file_list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_ip",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Read IP core files (.xci).",
            &["read_ip ?-quiet? file_list"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
