//! `ASN1::decode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASN1::decode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Decodes ASN.1 records.",
            &["ASN1::decode ELEMENT FORMAT (VAR_NAME)*"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
