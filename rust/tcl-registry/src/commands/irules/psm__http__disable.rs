//! `PSM::HTTP::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSM::HTTP::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "To disable PSM for HTTP traffic.",
            &["PSM::HTTP::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
