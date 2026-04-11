//! `qverilog` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "qverilog",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Questa one-step Verilog compile and simulate.",
            &["qverilog ?-sv? ?+define+name=val? ?-R? ?-c? file_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
