//! `remote_addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "remote_addr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Deprecated: Use IP::remote_addr instead.",
            &["remote_addr"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
