//! `local_addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "local_addr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Deprecated: Use IP::local_addr instead.",
            &["local_addr"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
