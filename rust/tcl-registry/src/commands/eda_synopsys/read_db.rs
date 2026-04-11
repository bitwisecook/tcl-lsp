//! `read_db` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_db",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Read a .db technology library.",
            &["read_db file_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
