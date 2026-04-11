//! `read_file` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_file",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Read a design file into memory.",
            &["read_file ?-format format? file_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
