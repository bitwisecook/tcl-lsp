//! `filter_collection` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "filter_collection",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Filter a collection by an expression.",
            &["filter_collection collection filter_expr"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
