//! `vcom` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[
    FormSpec { kind: FormKind::Default, synopsis: "vcom ?-work library? ?-2008? ?-explicit? ?-check_synthesis? ?-lint? ?-suppress n? ?-nowarn n? file_list" },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "vcom",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Compile VHDL source files.", &["vcom ?-work library? ?-2008? ?-explicit? ?-check_synthesis? ?-lint? ?-suppress n? ?-nowarn n? file_l"], "F5")),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
