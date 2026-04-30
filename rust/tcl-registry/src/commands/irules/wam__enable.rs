//! `WAM::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WAM::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enables Web Accelerator plugin processing on the connection.",
            &["WAM::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
