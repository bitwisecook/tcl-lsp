//! `timing` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "timing",
        traits: Traits::IRULES_TOP_LEVEL_ONLY,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enables or disables iRule timing statistics.",
            &["timing TIMING"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
