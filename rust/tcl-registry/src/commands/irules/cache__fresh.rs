//! `CACHE::fresh` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::fresh",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns state of freshness flag for request.",
            synopsis: &["CACHE::fresh"],
            snippet: "Returns 1 for cached document fresh, 0 otherwise.",
            source: "https://clouddocs.f5.com/api/irules/CACHE__fresh.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["CACHE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "CACHE::fresh",
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
