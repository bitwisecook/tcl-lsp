//! `insert_clock_gating` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "insert_clock_gating",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Insert clock gating logic.",
            &["insert_clock_gating ?-global?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
