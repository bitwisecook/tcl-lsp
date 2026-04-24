//! `export_assignments` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "export_assignments",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Export assignments to the .qsf file.",
            &["export_assignments"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
