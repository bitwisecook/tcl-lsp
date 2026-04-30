//! `create_constraint_mode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_constraint_mode",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create a constraint mode for MMMC.",
            &["create_constraint_mode -name name -sdc_files file_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
