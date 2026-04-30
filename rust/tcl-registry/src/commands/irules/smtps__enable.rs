//! `SMTPS::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SMTPS::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enable SMTPS (STARTTLS for SMTP).",
            &["SMTPS::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
