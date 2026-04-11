//! `PSM::HTTP::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSM::HTTP::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "To enable PSM for HTTP traffic.",
            &["PSM::HTTP::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
