//! `formal_compile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "formal_compile",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Compile design for formal verification.",
            &["formal_compile ?-d design? ?-work library?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
