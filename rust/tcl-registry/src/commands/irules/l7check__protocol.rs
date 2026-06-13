//! `L7CHECK::protocol` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "L7CHECK::protocol",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Set or get L7 protocol value.",
            synopsis: &["L7CHECK::protocol set VALUE", "L7CHECK::protocol get"],
            snippet: "The L7CHECK::protocol commands allow you to set or retrieve L7 protocol value.",
            source: "https://clouddocs.f5.com/api/irules/L7CHECK__protocol.html",
            examples: "when L7CHECK_CLIENT_DATA {\n    if { [L7CHECK::protocol get] == \"https\" } {\n        pool clients_https\n    } else {\n        pool clients_non_https\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["CONNECTOR", "L7CHECK"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "L7CHECK::protocol set VALUE" },
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
