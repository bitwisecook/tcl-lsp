//! `formal_verify` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "formal_verify",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Run formal verification.",
            &["formal_verify ?-timeout time? ?-engine engine?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
