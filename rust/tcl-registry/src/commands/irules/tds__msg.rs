//! `TDS::msg` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TDS::msg",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns TDS message data.",
            &["TDS::msg"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
