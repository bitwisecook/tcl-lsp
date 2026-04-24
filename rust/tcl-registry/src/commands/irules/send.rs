//! `send` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "send",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sends data on an existing sideband connection.",
            &["send (("],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
