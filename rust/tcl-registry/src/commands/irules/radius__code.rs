//! `RADIUS::code` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RADIUS::code",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns the RADIUS message code",
            synopsis: &["RADIUS::code"],
            snippet: "This command returns the RADIUS message code",
            source: "https://clouddocs.f5.com/api/irules/RADIUS__code.html",
            examples: "when CLIENT_ACCEPTED {\n    if { [RADIUS::code] == 4 } {\n        set rd 0\n        # Extract the APN information from the AVP\n        set called_station_id [RADIUS::avp 30 \"string\"]\n        if {$called_station_id == \"station1\"} {\n            set rd 1\n        } elseif {$called_station_id == \"station2\"} {\n            set rd 2\n        }\n        # Overwrite the default route domain value with the new value.\n        RADIUS::rtdom $rd\n    }\n}",
            return_value: "returns radius message code.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[
                "CLIENT_ACCEPTED",
                "CLIENT_CLOSED",
                "CLIENT_DATA",
                "SERVER_CLOSED",
                "SERVER_CONNECTED",
                "SERVER_DATA",
            ],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "RADIUS::code" },
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
