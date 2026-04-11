//! `PSM::FTP::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSM::FTP::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "To enable PSM for FTP traffic.",
            &["PSM::FTP::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
