//! `PROFILE::ftp` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::ftp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of an FTP profile setting.",
            &["PROFILE::ftp ATTR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
