//! `open_run` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "open_run",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Open a completed run in memory.",
            &["open_run ?-name name? run_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
