//! `set_size_only` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_size_only",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Mark cells as size-only (no restructuring).",
            &["set_size_only object_list ?-all_instances?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
