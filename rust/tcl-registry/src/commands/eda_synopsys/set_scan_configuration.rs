//! `set_scan_configuration` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_scan_configuration",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Configure scan chain parameters.",
            &["set_scan_configuration ?-chain_count n? ?-clock_mixing mix_type? ?-style style?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
