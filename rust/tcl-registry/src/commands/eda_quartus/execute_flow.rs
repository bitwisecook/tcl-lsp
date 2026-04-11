//! `execute_flow` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "execute_flow",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Execute a Quartus compilation flow.", &["execute_flow -compile | -analysis_and_synthesis | -fitter | -assembler | -timing_analyzer | -eda_net"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
