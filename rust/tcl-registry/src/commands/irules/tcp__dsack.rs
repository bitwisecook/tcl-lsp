//! `TCP::dsack` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::dsack",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Toggles TCP duplicate selective acknowledgments (D-SACK).",
            &["TCP::dsack BOOL_VALUE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
