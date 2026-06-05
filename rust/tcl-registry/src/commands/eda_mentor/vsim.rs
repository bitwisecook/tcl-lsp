//! `vsim` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[
    FormSpec { kind: FormKind::Default, synopsis: "vsim ?-c? ?-do command? ?-t time_resolution? ?-voptargs args? ?-L library? ?-debugdb? ?-wlf file? ?-onfinish action? ?-gui? ?work.top_module?" },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "vsim",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Load and start simulation.", &["vsim ?-c? ?-do command? ?-t time_resolution? ?-voptargs args? ?-L library? ?-debugdb? ?-wlf file? ?-"], "F5")),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
