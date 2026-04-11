//! `remove_cell` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "remove_cell",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Remove a cell from the design.",
            &["remove_cell cell_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
