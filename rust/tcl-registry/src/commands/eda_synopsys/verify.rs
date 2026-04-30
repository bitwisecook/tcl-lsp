//! `verify` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "verify",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Run formal equivalence checking.",
            &["verify ?-verbose?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
