//! `ISTATS::set` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ISTATS::set",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the given key's value within iStats.",
            &["ISTATS::set KEY VALUE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
