//! `LSN::address` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::address",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Explicitly set the translation address regardless of the configured LSN pool.",
            &["LSN::address TRANSLATION_ADDR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
