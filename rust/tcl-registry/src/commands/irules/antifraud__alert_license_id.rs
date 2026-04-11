//! `ANTIFRAUD::alert_license_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_license_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns crc32 of the license id in hex.",
            &["ANTIFRAUD::alert_license_id"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
