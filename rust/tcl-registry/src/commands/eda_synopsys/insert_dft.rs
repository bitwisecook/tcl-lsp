//! `insert_dft` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "insert_dft",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Insert DFT structures (scan chains).",
            &["insert_dft"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
