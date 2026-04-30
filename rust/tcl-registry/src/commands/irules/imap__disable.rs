//! `IMAP::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IMAP::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disable IMAP protocol handler.",
            &["IMAP::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
