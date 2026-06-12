//! `DIAMETER::is_request` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::is_request",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns true if the current message is a DIAMETER request.",
            synopsis: &["DIAMETER::is_request"],
            snippet: "This iRule command returns true if the current message is a DIAMETER request.\nOtherwise, it returns false.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__is_request.html",
            examples: "when DIAMETER_INGRESS {\n    if { [DIAMETER::is_request] } {\n        log local0. \"Request received\"\n    }\n}",
            return_value: "TRUE or FALSE",
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
            FormSpec { kind: FormKind::Default, synopsis: "DIAMETER::is_request" },
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
