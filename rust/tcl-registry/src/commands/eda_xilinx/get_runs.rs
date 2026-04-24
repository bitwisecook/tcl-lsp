//! `get_runs` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_runs",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get run objects.",
            &["get_runs ?-regexp? ?-filter expr? ?patterns?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
