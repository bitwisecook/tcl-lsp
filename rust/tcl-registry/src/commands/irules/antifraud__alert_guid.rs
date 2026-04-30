//! `ANTIFRAUD::alert_guid` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_guid",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns GUID that is used to identify which users have been infected with malwar",
            &["ANTIFRAUD::alert_guid"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
