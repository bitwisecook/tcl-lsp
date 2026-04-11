//! `ONECONNECT::label` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ONECONNECT::label",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Associate OneConnect keying information with connection.",
            &["ONECONNECT::label update KEY"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
