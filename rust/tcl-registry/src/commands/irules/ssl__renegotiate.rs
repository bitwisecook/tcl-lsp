//! `SSL::renegotiate` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::renegotiate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Controls renegotiation of an SSL connection.",
            &["SSL::renegotiate (enable | disable)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
