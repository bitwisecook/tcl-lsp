//! `syn_generic` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "syn_generic",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Perform technology-independent synthesis.",
            &["syn_generic ?-effort effort?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
