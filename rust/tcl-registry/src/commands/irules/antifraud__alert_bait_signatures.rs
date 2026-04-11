//! `ANTIFRAUD::alert_bait_signatures` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_bait_signatures",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Deprecated: For the trojan_bait alert: returns the bait signatures in an escaped",
            &["ANTIFRAUD::alert_bait_signatures"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
