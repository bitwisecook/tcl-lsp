//! `WAM::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WAM::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables Web Accelerator plugin processing on the connection.",
            &["WAM::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
