//! `TCP::nagle` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::nagle",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Toggles the Nagle mode.",
            &["TCP::nagle (enable | disable | auto)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
