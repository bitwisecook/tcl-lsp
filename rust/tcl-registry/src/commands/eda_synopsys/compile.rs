//! `compile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "compile",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Compile (synthesize) the current design.", &["compile ?-map_effort effort? ?-area_effort effort? ?-incremental_mapping? ?-exact_map? ?-no_design_r"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
