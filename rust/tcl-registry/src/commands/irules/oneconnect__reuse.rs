//! `ONECONNECT::reuse` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ONECONNECT::reuse",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Controls server-side connection reuse.",
            &["ONECONNECT::reuse (BOOL_VALUE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
