//! `AUTH::cert_credential` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::cert_credential",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the peer certificate credential to the value of a peer certificate for a fu",
            &["AUTH::cert_credential AUTH_ID PEER_CERTIFICATE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
