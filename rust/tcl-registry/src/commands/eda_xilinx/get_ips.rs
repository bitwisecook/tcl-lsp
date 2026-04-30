//! `get_ips` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_ips",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get IP core instances.",
            &["get_ips ?-regexp? ?-nocase? ?-filter expr? ?patterns?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
