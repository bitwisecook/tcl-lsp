//! `AUTH::cert_issuer_credential` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::cert_issuer_credential",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the peer certificate issuer credential to the value of for a future AUTH::a",
            &["AUTH::cert_issuer_credential AUTH_ID PEER_CERTIFICATE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
