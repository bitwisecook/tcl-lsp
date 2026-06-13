//! `DIAMETER::state` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::state",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the current state of the Diameter peer's connection.",
            synopsis: &["DIAMETER::state"],
            snippet: "This iRule command returns the current state of the Diameter peer\\'s\nconnection, as a string. There are five possible states:\n  * CLOSED - The connection is down\n  * WAIT_ICEA - still waiting for the initial CEA\n  * ROPEN - The connection has been reopened\n  * IOPEN - The connection is open for the first time\n  * CLOSING - The connection will soon be down",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__state.html",
            examples: "when DIAMETER_INGRESS {\n    if { [DIAMETER::state] == \"ROPEN\" } {\n        log local0. \"Received a DIAMETER message via a reopened connection\"\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER", "MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DIAMETER::state" },
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
