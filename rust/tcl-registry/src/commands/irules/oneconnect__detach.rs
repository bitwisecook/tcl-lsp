//! `ONECONNECT::detach` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ONECONNECT::detach",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Detaches server-side OneConnect connections.",
            &["ONECONNECT::detach BOOL_VALUE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
