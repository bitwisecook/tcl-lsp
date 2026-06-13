//! `DIAMETER::dynamic_route_insertion` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::dynamic_route_insertion",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Set whether dynamic route insertion is enabled.",
            synopsis: &["DIAMETER::dynamic_route_insertion ( BOOLEAN )?"],
            snippet: "If status is set to \"enabled\", a dynamic route will be created for this connection.\n\nThis value, once set, remains for the life of the connection.  After the connection is closed, this route will be removed once \"timeout\" seconds have elapsed.  The default timeout is set by the configuration option \"dynamic-route-timeout\".\n\nThe zero-argument form of this command returns whether the setting is enabled on the current connection.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__dynamic_route_insertion.html",
            examples: "when CLIENT_ACCEPTED {\n                if { ([IP::address] starts_with \"192.168.\") } {\n                    DIAMETER::dynamic_route_insertion disabled\n                }\n            }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["CLIENT_ACCEPTED", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DIAMETER::dynamic_route_insertion ( BOOLEAN )?" },
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
