//! `ANTIFRAUD::username` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::username",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns or sets username value, only in context of ANTIFRAUD_LOGIN event.",
            &["ANTIFRAUD::username (USERNAME_ALIAS)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
