//! `write` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "write",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Write design data to a file.",
            &["write ?-format format? ?-hierarchy? ?-output file? ?design_list?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
