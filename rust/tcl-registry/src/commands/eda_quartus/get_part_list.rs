//! `get_part_list` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "get_part_list ?-family family? ?-speed_grade speed_grade?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_part_list",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get a list of available FPGA parts.",
            &["get_part_list ?-family family? ?-speed_grade speed_grade?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
