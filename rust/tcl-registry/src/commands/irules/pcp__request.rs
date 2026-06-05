//! `PCP::request` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PCP::request",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Provides access to the data sent in a PCP request.",
            synopsis: &["PCP::request (opcode |"],
            snippet: "This command provides access to the data sent in a PCP (Port Control\nProtocol) request. Access to this data is read-only, and the data in\nthe PCP request cannot be modified via the PCP::request command.",
            source: "https://clouddocs.f5.com/api/irules/PCP__request.html",
            examples: "when PCP_REQUEST {\n     if {[PCP::request opcode] == \"map\" && [PCP::request client-addr] == \"192.168.1.1\" } {\n         log \"Received PCP map request for port [PCP::request internal-port] from 192.168.1.1\"\n     }\n}",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "PCP::request (opcode |" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
