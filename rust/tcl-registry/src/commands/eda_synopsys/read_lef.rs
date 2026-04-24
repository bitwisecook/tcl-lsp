//! `read_lef` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_lef",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Read a LEF technology file.",
            &["read_lef file_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
