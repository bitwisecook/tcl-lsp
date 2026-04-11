//! `compile_ultra` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "compile_ultra",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Compile with advanced optimizations.", &["compile_ultra ?-incremental? ?-retime? ?-scan? ?-no_autoungroup? ?-no_boundary_optimization? ?-gate_"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
