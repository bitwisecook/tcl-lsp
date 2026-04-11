//! `synth_design` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "synth_design",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Run synthesis.", &["synth_design ?-top top_module? ?-part part? ?-flatten_hierarchy value? ?-fanout_limit n? ?-retiming?"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
