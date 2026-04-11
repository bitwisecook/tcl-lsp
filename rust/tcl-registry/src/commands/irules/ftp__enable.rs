//! `FTP::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FTP::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enable FTP protocol handler.",
            &["FTP::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
