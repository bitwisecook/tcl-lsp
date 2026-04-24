//! `optimize_netlist` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "optimize_netlist",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Perform incremental netlist optimization.",
            &["optimize_netlist -area"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
