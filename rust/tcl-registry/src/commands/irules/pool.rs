//! `pool` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "pool",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet {
            summary: "Select a load-balancing pool for the current flow.",
            synopsis: &["pool pool_name ?member_addr member_port?"],
            snippet: "Can direct traffic to a pool, optionally pinning to a specific member.",
            source: "https://clouddocs.f5.com/api/irules/pool.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "pool pool_name ?member_addr member_port?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Server,
        }],
        ..CommandSpec::DEFAULT
    }
}
