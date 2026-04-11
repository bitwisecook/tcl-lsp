//! `define_proc_attributes` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "define_proc_attributes",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Define attributes for a procedure.",
            &["define_proc_attributes proc_name ?-info string?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
