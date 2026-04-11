//! `POP3::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "POP3::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disable POP3 (STARTTLS for POP3).",
            &["POP3::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
