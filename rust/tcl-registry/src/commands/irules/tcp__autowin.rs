//! `TCP::autowin` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::autowin",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Toggles automatic window tuning.",
            &["TCP::autowin BOOL_VALUE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
