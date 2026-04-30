//! `HSL::send` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HSL::send",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sends data via High Speed Logging.",
            &["HSL::send HANDLE DATA"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
