//! `verify_drc` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "verify_drc",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Run design rule checking.",
            &["verify_drc ?-limit n?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
