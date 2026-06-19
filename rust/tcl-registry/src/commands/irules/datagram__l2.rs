//! `DATAGRAM::l2` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DATAGRAM::l2",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns Layer 2 destination address.",
            synopsis: &["DATAGRAM::l2 dest"],
            snippet: "This iRules command returns the L2 destination of an ingress frame.\n\nDATAGRAM::l2 dest",
            source: "https://clouddocs.f5.com/api/irules/DATAGRAM__l2.html",
            examples: "when FLOW_INIT {\n     set l2_dest [ DATAGRAM::l2 dest ]\n     log local0. \"L2 destination MAC $l2_dest\"\n}",
            return_value: "Returns the L2 destination of an ingress frame.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FLOW"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DATAGRAM::l2 dest",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
