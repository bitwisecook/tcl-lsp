//! `change` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "change",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Change the value of a VHDL signal or variable.",
            &["change signal_name value"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
