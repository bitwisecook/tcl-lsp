//! `PSM::HTTP::enable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSM::HTTP::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "To enable PSM for HTTP traffic.",
            synopsis: &["PSM::HTTP::enable"],
            snippet: "To enable PSM for HTTP traffic",
            source: "https://clouddocs.f5.com/api/irules/PSM__HTTP__enable.html",
            examples: "when HTTP_REQUEST {\n    PSM::HTTP::disable\n    if { [HTTP::uri] starts_with \"/enforce\" } {\n        PSM::HTTP::enable\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &["CLIENT_ACCEPTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "PSM::HTTP::enable" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
