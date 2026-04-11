//! `size_cell` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "size_cell",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Resize a cell to a different library cell.",
            &["size_cell cell_name lib_cell"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
