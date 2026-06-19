//! `FTP::enforce_tls_session_reuse` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "FTP::enforce_tls_session_reuse",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set the state of enforcing TLS session reuse.",
            synopsis: &["FTP::enforce_tls_session_reuse (enable | disable)?"],
            snippet: "Enable or disable enforcing TLS session reuse, when enabled, Bigip rejects the data connection if it fails to reuse existed TLS session. Returns the current status if no option is specified.",
            source: "https://clouddocs.f5.com/api/irules/FTP__enforce_tls_session_reuse.html",
            examples: "when CLIENT_ACCEPTED {\n                FTP::enforce_tls_session_reuse enable\n            }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "FTP::enforce_tls_session_reuse (enable | disable)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::FtpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
