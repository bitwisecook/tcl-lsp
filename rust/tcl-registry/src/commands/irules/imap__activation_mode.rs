//! `IMAP::activation_mode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IMAP::activation_mode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set the activation mode for IMAP STARTTLS.",
            &["IMAP::activation_mode (none | allow | require)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
