//! `create_generated_clock` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[
    FormSpec { kind: FormKind::Default, synopsis: "create_generated_clock ?-name name? -source master_pin ?-edges edge_list? ?-divide_by factor? ?-multiply_by factor? ?-duty_cycle percent? ?-invert? ?-add? source_objects" },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_generated_clock",
        dialects: Some(DialectSet::SYNOPSYS | DialectSet::CADENCE | DialectSet::XILINX | DialectSet::QUARTUS | DialectSet::MENTOR),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief("Create a generated clock object.", &["create_generated_clock ?-name name? -source master_pin ?-edges edge_list? ?-divide_by factor? ?-mult"], "F5")),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
