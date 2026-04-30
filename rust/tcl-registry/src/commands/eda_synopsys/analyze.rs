//! `analyze` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "analyze",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Analyze HDL source files for syntax and semantic errors.", &["analyze -format format ?-library lib? ?-define define_list? ?-work work? file_list"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
