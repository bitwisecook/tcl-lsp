//! `PCP::response` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PCP::response",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Provides access to the data in a PCP response packet.",
            synopsis: &["PCP::response (opcode |"],
            snippet: "This command provides access to the data in a PCP (Port Control\nProtocol) response packet. Access to this data is read-only, and the\ndata in the PCP response cannot be modified via the PCP::response\ncommand.",
            source: "https://clouddocs.f5.com/api/irules/PCP__response.html",
            examples: "when PCP_RESPONSE {\n    if {[PCP::response opcode] == \"map\" && [PCP::response result] != 0] } {\n        log \"PCP map request from\\\n              [PCP::response client-addr]:[PCP::response internal-port]\\\n              failed with a result of [PCP::response result]\"\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["PCP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "PCP::response (opcode |",
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
