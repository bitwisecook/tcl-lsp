//! `set_false_path` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[
    FormSpec { kind: FormKind::Default, synopsis: "set_false_path ?-setup | -hold? ?-from from_list? ?-through through_list? ?-to to_list? ?-rise_from rise_from_list? ?-fall_from fall_from_list? ?-rise_to rise_to_list? ?-fall_to fall_to_list? ?-comment comment?" },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_false_path",
        dialects: Some(DialectSet::SYNOPSYS | DialectSet::CADENCE | DialectSet::XILINX | DialectSet::QUARTUS | DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Define a false timing path.", &["set_false_path ?-setup | -hold? ?-from from_list? ?-through through_list? ?-to to_list? ?-rise_from "], "F5")),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
