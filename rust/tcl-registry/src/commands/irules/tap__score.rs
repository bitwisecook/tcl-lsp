//! `TAP::score` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TAP::score",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or updates risk score.",
            synopsis: &["TAP::score (SCORE)?"],
            snippet: "If score specified sets supplied score. Returns previous score.",
            source: "https://clouddocs.f5.com/api/irules/TAP__score.html",
            examples: "when TAP_REQUEST {\n    if {    ([TAP::score] > 85) } {\n        drop\n    }\n}",
            return_value: "Returns an integer value from (0 to 100). If supplied score to set function returns previous score.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["TAP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TAP::score (SCORE)?" },
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
