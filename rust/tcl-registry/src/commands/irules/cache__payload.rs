//! `CACHE::payload` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the HTTP payload of the cache response.",
            synopsis: &["CACHE::payload"],
            snippet: "Returns the HTTP payload of the cache response.\n\nCACHE::payload\n\n     * Returns the HTTP payload of the cache response.",
            source: "https://clouddocs.f5.com/api/irules/CACHE__payload.html",
            examples: "when CACHE_RESPONSE {\n  set payload [CACHE::payload]\n}",
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
            synopsis: "CACHE::payload",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::StreamProfile,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
