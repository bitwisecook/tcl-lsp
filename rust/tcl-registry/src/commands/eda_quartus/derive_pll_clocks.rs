//! `derive_pll_clocks` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "derive_pll_clocks",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Automatically derive PLL output clocks.",
            &["derive_pll_clocks ?-create_base_clocks? ?-use_net_name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
