//! `FTP::enforce_tls_session_reuse` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FTP::enforce_tls_session_reuse",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set the state of enforcing TLS session reuse.",
            &["FTP::enforce_tls_session_reuse (enable | disable)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
