//! `vsim` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "vsim",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Load and start simulation.", &["vsim ?-c? ?-do command? ?-t time_resolution? ?-voptargs args? ?-L library? ?-debugdb? ?-wlf file? ?-"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
