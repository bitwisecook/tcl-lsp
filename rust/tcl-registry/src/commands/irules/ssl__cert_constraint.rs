//! `SSL::cert_constraint` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::cert_constraint",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Inserts cert constraint information to the certificate.",
            synopsis: &["SSL::cert_constraint (ARG ARG)"],
            snippet: "Inserts a certificate extension to the certificate.",
            source: "https://clouddocs.f5.com/api/irules/SSL__cert_constraint.html",
            examples: "when CLIENTSSL_HANDSHAKE {\n    log local0.info \"CLIENTSSL_HANDSHAKE\"\n    SSL::cert_constraint 1.2.3.4.5 \"This is the oid-value of 1.2.3.4.5\"\n}",
            return_value: "SSL::cert_constraint <oid oid-value> Inserts the <oid oid-value> as an extension with OID=oid and value=oid-value to the certificate.",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["CLIENTSSL"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
