//! `SSL::is_renegotiation_secure` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::is_renegotiation_secure",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the current state of SSL Secure Renegotiation.",
            &["SSL::is_renegotiation_secure"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
