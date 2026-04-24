//! `VDI::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "VDI::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disable VDI plugin.",
            &["VDI::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
