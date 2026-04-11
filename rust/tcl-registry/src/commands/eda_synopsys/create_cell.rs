//! `create_cell` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_cell",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Create a new cell instance.",
            &["create_cell cell_name lib_cell"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
