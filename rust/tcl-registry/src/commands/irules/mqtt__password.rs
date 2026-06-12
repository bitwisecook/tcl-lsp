//! `MQTT::password` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::password",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get or set password field of MQTT CONNECT message.",
            synopsis: &["MQTT::password (PASSWORD)?"],
            snippet: "This command can be used to get or set password field of MQTT message.\nThis command is valid only for following MQTT message types:\n\n    CONNECT",
            source: "https://clouddocs.f5.com/api/irules/MQTT__password.html",
            examples: "# Reject connections with no password\nwhen MQTT_CLIENT_INGRESS {\n    set type [MQTT::type]\n    switch $type {\n        \"CONNECT\" {\n            if {[MQTT::username] == \"\" } {\n               MQTT::respond type CONNACK return_code 4\n               MQTT::disconnect\n            }\n        }\n    }\n}",
            return_value: "When called without an argument, this command returns the password field of MQTT CONNECT message.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["MQTT"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MQTT::password (PASSWORD)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
