//! `connect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "connect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Establishes a sideband connection.",
            &["connect info ("],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
