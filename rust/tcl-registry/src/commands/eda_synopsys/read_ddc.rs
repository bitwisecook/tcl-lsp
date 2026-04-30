//! `read_ddc` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_ddc",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Read a Synopsys DDC database.",
            &["read_ddc file_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
