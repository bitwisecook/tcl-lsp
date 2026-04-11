//! `syn_opt` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "syn_opt",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Perform post-map optimization.",
            &["syn_opt ?-effort effort? ?-incremental?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
