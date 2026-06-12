//! `node` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "node",
        traits: Traits::CSE_CANDIDATE.union(Traits::DIAGRAM_ACTION),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Route traffic directly to a specific node.",
            synopsis: &["node ip_addr ?service_port?"],
            snippet: "Bypasses pool selection and targets an explicit backend endpoint.",
            source: "https://clouddocs.f5.com/api/irules/node.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["PERSIST_DOWN"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "node ip_addr ?service_port?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NodeSelection,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Server,
        }],
        ..CommandSpec::DEFAULT
    }
}
