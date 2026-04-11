//! `ANTIFRAUD::guid` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::guid",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns GUID value, only in context of ANTIFRAUD_LOGIN event.",
            &["ANTIFRAUD::guid"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
