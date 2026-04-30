//! `VDI::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "VDI::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enable VDI plugin.",
            &["VDI::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
