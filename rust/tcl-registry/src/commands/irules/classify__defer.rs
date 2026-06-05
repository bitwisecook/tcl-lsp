//! `CLASSIFY::defer` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFY::defer",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Defers the classification of the flow to response.",
            synopsis: &["CLASSIFY::defer"],
            snippet: "This command defers the classification of the flow to response (for\nHTTP - response message, for non-HTTP, opposite traffic direction from\ntraffic initiator)\n\n* Note: APM / AFM / PEM license is required for functionality to work.\n\nCLASSIFY::defer",
            source: "https://clouddocs.f5.com/api/irules/CLASSIFY__defer.html",
            examples: "",
            return_value: "",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "CLASSIFY::defer" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ClassificationState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
