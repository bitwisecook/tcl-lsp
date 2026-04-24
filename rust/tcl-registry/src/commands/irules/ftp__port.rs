//! `FTP::port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FTP::port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Controls the range of passive mode FTP ephemeral ports.",
            &["FTP::port FIRST (LAST)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
