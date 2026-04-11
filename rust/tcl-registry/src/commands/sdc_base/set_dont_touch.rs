//! `set_dont_touch` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_dont_touch",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Prevent optimization of cells/nets.",
            &["set_dont_touch object_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
