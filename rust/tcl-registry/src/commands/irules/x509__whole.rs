//! `X509::whole` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::whole",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns an X509 certificate in PEM format.",
            synopsis: &["X509::whole CERTIFICATE"],
            snippet: "Returns the specified X509 certificate, in its entirety, in PEM format.",
            source: "https://clouddocs.f5.com/api/irules/X509__whole.html",
            examples: "when CLIENTSSL_CLIENTCERT {\n  set client_cert [SSL::cert 0]\n  log local0. \"[X509::whole $client_cert]\"\n}",
            return_value: "Returns an X509 certificate in PEM format.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "X509::whole CERTIFICATE" },
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
