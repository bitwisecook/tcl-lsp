//! `SSL::cert_constraint` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::cert_constraint",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Inserts cert constraint information to the certificate.",
            &["SSL::cert_constraint (ARG ARG)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
