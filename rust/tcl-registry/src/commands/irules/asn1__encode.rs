//! `ASN1::encode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASN1::encode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Encodes ASN.1 records.",
            &["ASN1::encode ('BER' | 'DER') FORMAT (VALUE)*"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
