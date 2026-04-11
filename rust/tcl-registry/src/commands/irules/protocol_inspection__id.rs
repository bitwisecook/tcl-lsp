//! `PROTOCOL_INSPECTION::id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROTOCOL_INSPECTION::id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Provides protocol inspection match result.",
            &["PROTOCOL_INSPECTION::id"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
