//! `sizeof_collection` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "sizeof_collection",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the size of a collection.",
            &["sizeof_collection collection"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
