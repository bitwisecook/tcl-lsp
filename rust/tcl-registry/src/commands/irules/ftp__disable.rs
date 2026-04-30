//! `FTP::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FTP::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disable FTP protocol handler.",
            &["FTP::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
