//! `X509::not_valid_before` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::not_valid_before",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the not-valid-before date of an X509 certificate.",
            synopsis: &["X509::not_valid_before CERTIFICATE"],
            snippet: "Returns the not-valid-before date of the specified X509 certificate.",
            source: "https://clouddocs.f5.com/api/irules/X509__not_valid_before.html",
            examples: "when SERVERSSL_HANDSHAKE {\n  set server_cert [SSL::cert 0]\n  log local0. \"Server Certificate Valid Date -\n   [X509::not_valid_before $server_cert] -\n   [X509::not_valid_after $server_cert]\"\n}",
            return_value: "Returns the not-valid-before date of an X509 certificate.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "X509::not_valid_before CERTIFICATE" },
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
