//! `SMTPS::activation_mode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SMTPS::activation_mode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set the activation mode.",
            &["SMTPS::activation_mode (none | allow | require)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
