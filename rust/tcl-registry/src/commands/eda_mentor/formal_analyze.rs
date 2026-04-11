//! `formal_analyze` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "formal_analyze",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Analyze formal verification results.",
            &["formal_analyze ?-property prop_list?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
