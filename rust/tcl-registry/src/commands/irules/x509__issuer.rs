//! `X509::issuer` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::issuer",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the issuer of an X509 certificate.",
            synopsis: &["X509::issuer CERTIFICATE"],
            snippet: "Returns the issuer of the specified X509 certificate.",
            source: "https://clouddocs.f5.com/api/irules/X509__issuer.html",
            examples: "when SERVERSSL_HANDSHAKE {\n  set ssl_cert [SSL::cert 0]\n  log local0. \"Cert issuer - [X509::issuer $ssl_cert]\"\n}",
            return_value: "Returns the issuer of an X509 certificate.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "X509::issuer CERTIFICATE",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
        }],
        ..CommandSpec::DEFAULT
    }
}
