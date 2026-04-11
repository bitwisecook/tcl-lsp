//! `wait_on_run` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "wait_on_run",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Wait for a run to complete.",
            &["wait_on_run ?-timeout minutes? run_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
