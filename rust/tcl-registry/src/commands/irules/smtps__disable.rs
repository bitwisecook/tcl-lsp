//! `SMTPS::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SMTPS::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disable SMTPS (STARTTLS for SMTP).",
            &["SMTPS::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
