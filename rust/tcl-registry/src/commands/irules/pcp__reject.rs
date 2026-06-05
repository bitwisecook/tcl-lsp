//! `PCP::reject` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PCP::reject",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Provides the ability to cause a PCP reqeust to fail based on processing in the iRule.",
            synopsis: &["PCP::reject PCP_RESULT_CODE"],
            snippet: "This command provides the ability to cause a PCP (Port Control\nProtocol) reqeust to fail based on processing in the iRule. If the\nreject command is issued, the PCP request is rejected with the\nspecified result code and no other action is taken by the PCP proxy.",
            source: "https://clouddocs.f5.com/api/irules/PCP__reject.html",
            examples: "when PCP_REQUEST {\n     if {[PCP::request opcode] == \"map\" &&\n             [PCP::request internal-port] == 22 } {\n         log \"Rejecting PCP request to map SSH\"\n         PCP::reject 1\n    }\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "PCP::reject PCP_RESULT_CODE" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ConnectionControl,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
