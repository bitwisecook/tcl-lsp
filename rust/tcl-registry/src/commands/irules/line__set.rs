//! `LINE::set` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LINE::set",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `LINE::set`.",
            &["LINE::set"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
