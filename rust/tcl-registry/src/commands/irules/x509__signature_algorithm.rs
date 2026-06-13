//! `X509::signature_algorithm` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::signature_algorithm",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the signature algorithm of an X509 certificate.",
            synopsis: &["X509::signature_algorithm CERTIFICATE"],
            snippet: "Returns the signature algorithm of the specified X509 certificate.",
            source: "https://clouddocs.f5.com/api/irules/X509__signature_algorithm.html",
            examples: "when SERVERSSL_HANDSHAKE {\n    set ssl_cert [SSL::cert 0]\n    log local0. \"SIGNATURE ALGORITHM: [X509::signature_algorithm $ssl_cert]\"\n}",
            return_value: "Returns the signature algorithm of an X509 certificate.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "X509::signature_algorithm CERTIFICATE" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::SslState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
