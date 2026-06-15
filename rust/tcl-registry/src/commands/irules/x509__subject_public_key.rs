//! `X509::subject_public_key` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::subject_public_key",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the subjectXs public key of an X509 certificate.",
            synopsis: &["X509::subject_public_key (type | bits | curve_name)? CERTIFICATE"],
            snippet: "Returns the subject’s public key of the specified X509 certificate.",
            source: "https://clouddocs.f5.com/api/irules/X509__subject_public_key.html",
            examples: "when CLIENTSSL_CLIENTCERT {\n  set client_cert [SSL::cert 0]\n  log local0. \"Cert subject - [X509::subject $client_cert]\"\n  log local0. \"Cert public key - [X509::subject_public_key $client_cert]\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "X509::subject_public_key (type | bits | curve_name)? CERTIFICATE",
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
