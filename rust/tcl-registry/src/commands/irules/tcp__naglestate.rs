//! `TCP::naglestate` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::naglestate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns current state of Nagle algorithm.",
            &["TCP::naglestate"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
