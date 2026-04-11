//! `qvhdl` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "qvhdl",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Questa one-step VHDL compile and simulate.",
            &["qvhdl ?-2008? ?-R? ?-c? file_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
