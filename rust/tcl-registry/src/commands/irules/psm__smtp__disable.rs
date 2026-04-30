//! `PSM::SMTP::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSM::SMTP::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "To disable PSM for SMTP traffic.",
            &["PSM::SMTP::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
