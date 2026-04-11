//! `IMAP::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IMAP::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enable IMAP protocol handler.",
            &["IMAP::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
