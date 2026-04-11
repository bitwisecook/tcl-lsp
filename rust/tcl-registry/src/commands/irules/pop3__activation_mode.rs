//! `POP3::activation_mode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "POP3::activation_mode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set the activation mode.",
            &["POP3::activation_mode (none | allow | require)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
