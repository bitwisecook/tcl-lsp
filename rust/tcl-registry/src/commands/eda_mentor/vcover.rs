//! `vcover` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "vcover",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Process coverage data files.", &["vcover ?merge | report | attr? ?-input file_list? ?-output file? ?-detail? ?-verbose?"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
