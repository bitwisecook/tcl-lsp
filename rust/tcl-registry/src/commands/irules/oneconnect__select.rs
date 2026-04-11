//! `ONECONNECT::select` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ONECONNECT::select",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Instruct the proxy to use persistence data as a OneConnect keying label when con",
            &["ONECONNECT::select (persist | none)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
