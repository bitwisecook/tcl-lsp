//! `reset_run` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "reset_run",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Reset a run to its initial state.",
            &["reset_run run_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
