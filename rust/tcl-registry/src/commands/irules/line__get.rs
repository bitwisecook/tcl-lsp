//! `LINE::get` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LINE::get",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `LINE::get`.",
            &["LINE::get"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
