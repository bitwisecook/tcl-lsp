//! `POP3::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "POP3::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enable POP3 (STARTTLS for POP3).",
            &["POP3::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
