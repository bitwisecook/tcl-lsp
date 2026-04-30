//! `ASN1::element` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASN1::element",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns ASN1.1 record elements.",
            &["ASN1::element init ('BER' | 'DER')"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
