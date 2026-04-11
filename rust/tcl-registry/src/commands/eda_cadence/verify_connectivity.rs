//! `verify_connectivity` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "verify_connectivity",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Verify design connectivity.",
            &["verify_connectivity ?-type type? ?-error n?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
