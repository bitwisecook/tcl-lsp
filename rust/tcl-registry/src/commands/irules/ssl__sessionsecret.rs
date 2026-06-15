//! `SSL::sessionsecret` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::sessionsecret",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Return data about SSL handshake master secret.",
            synopsis: &["SSL::sessionsecret"],
            snippet: "Return data about SSL handshake master secret.",
            source: "https://clouddocs.f5.com/api/irules/SSL__sessionsecret.html",
            examples: "when CLIENTSSL_HANDSHAKE {\n    log local0. \"ClientSSL: Secret for [SSL::sessionid] is -> [SSL::sessionsecret]\"\n}",
            return_value: "SSL::sessionsecret Returns the current SSL handshake master secret.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["CLIENTSSL", "SERVERSSL"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SSL::sessionsecret",
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
