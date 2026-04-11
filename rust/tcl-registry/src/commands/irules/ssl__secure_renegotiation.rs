//! `SSL::secure_renegotiation` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::secure_renegotiation",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Controls the SSL Secure Renegotiation mode.",
            &["SSL::secure_renegotiation (request | require | require-strict)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
