//! `SSL::handshake` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::handshake",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Halts or resumes SSL activity.",
            synopsis: &["SSL::handshake (hold | resume)"],
            snippet: "Halts or resumes SSL activity. This is useful for suspending SSL activity while authentication is in progress.",
            source: "https://clouddocs.f5.com/api/irules/SSL__handshake.html",
            examples: "when AUTH_ERROR {\n    if {$auth_ldap_sid eq [AUTH::last_event_session_id]} {\n        reject\n    }\n}",
            return_value: "SSL::handshake hold Halts any SSL activity. Typically used when an authentication request is made.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSL::handshake (hold | resume)" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::SslState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
