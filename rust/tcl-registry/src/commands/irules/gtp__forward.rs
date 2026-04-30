//! `GTP::forward` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::forward",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Forwards GTP message to peer flow.",
            &["GTP::forward MESSAGE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
