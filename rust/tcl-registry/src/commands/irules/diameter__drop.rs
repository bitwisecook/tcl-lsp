//! `DIAMETER::drop` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::drop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Drops the current message quietly.",
            synopsis: &["DIAMETER::drop"],
            snippet: "This iRule command drops the current Diameter message quietly.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__drop.html",
            examples: "when DIAMETER_INGRESS {\n    if { [DIAMETER::command 275] && [DIAMETER::is_request] } {\n        DIAMETER::drop\n    }\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "DIAMETER::drop" },
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
