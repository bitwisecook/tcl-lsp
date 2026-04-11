//! `write_gds` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "write_gds",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Write a GDSII stream file.",
            &["write_gds ?-merge gds_files? ?-map_file map? file_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
