//! `ANTIFRAUD::alert_details` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_details",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns or sets alert details.",
            &["ANTIFRAUD::alert_details (VALUE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
