//! `ANTIFRAUD::alert_component` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_component",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns or sets error type according to alert_type.",
            &["ANTIFRAUD::alert_component (VALUE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
