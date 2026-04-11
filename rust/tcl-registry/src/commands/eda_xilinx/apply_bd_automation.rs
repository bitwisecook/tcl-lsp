//! `apply_bd_automation` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "apply_bd_automation",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Apply block design automation rules.",
            &["apply_bd_automation -rule rule_name ?-config config?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
