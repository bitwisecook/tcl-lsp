//! `set_wire_load_model` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "set_wire_load_model -name model_name ?-library lib? ?object_list?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_wire_load_model",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set the wire load model.",
            &["set_wire_load_model -name model_name ?-library lib? ?object_list?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
