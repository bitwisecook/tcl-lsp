//! `struct::queue` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "struct::queue",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate FIFO queue objects.",
            &["struct::queue ?queueName? ?=|:=|as|deserialize source?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
