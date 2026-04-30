//! `ANTIFRAUD::disable_app_layer_encryption` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::disable_app_layer_encryption",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables application layer encryption for the current transaction.",
            &["ANTIFRAUD::disable_app_layer_encryption"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
