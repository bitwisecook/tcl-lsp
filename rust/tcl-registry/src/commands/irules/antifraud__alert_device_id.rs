//! `ANTIFRAUD::alert_device_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_device_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Deprecated: Returns flash GUID.",
            &["ANTIFRAUD::alert_device_id"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
