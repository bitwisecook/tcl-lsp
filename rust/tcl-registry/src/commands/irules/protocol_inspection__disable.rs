//! `PROTOCOL_INSPECTION::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROTOCOL_INSPECTION::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables inspection match of the flow.",
            &["PROTOCOL_INSPECTION::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
