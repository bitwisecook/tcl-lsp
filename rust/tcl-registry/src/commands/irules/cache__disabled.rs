//! `CACHE::disabled` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::disabled",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns state of cache disable flag",
            synopsis: &["CACHE::disabled"],
            snippet: "Returns 1 for cache disabled, 0 otherwise.\n\nCACHE::disabled\n\n     * Returns true (1) or false (0) to indicate state of cache disable flag.",
            source: "https://clouddocs.f5.com/api/irules/CACHE__disabled.html",
            examples: "when HTTP_RESPONSE {\n    set val [CACHE::disabled]\n    log local0. \"Cache disable state: $val\"\n}",
            return_value: "Returns 0 or 1.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "CACHE::disabled",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::StreamProfile,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
