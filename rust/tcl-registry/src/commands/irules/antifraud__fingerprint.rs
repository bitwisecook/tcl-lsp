//! `ANTIFRAUD::fingerprint` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::fingerprint",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns fingerprint data, only in context of ANTIFRAUD_LOGIN event.",
            &["ANTIFRAUD::fingerprint"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
