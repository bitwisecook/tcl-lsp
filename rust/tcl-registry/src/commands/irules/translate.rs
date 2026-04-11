//! `translate` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "translate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enables, disables, or queries (as specified) destination address or port transla",
            &["translate (address | port | service)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
