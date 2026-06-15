//! `SSL::is_renegotiation_secure` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::is_renegotiation_secure",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the current state of SSL Secure Renegotiation.",
            synopsis: &["SSL::is_renegotiation_secure"],
            snippet: "Returns the current state of SSL Secure Renegotiation.",
            source: "https://clouddocs.f5.com/api/irules/SSL__is_renegotiation_secure.html",
            examples: "when CLIENTSSL_SERVERHELLO_SEND {\n    set secure_renegotiation_enabled [SSL::is_renegotiation_secure]\n}",
            return_value: "SSL::is_renegotiation_secure",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SSL::is_renegotiation_secure",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
