//! `MQTT::client_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::client_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get or set client identifier of MQTT CONNECT message",
            synopsis: &["MQTT::client_id (CLIENTID)?"],
            snippet: "This command can be used to get or set client identifier of MQTT message.\nThis command is valid only for following MQTT message types:\n\n    CONNECT",
            source: "https://clouddocs.f5.com/api/irules/MQTT__client_id.html",
            examples: "# Block connections from clientid in the blacklist_clientid_datagroup\nwhen MQTT_CLIENT_INGRESS {\n   set type [MQTT::type]\n   switch $type {\n       \"CONNECT\" {\n           set cid [MQTT::client_id]\n           if { [class exists blacklist_clientid_datagroup] } {\n               if {[class match  $cid equals blacklist_clientid_datagroup] != \"\"} {\n                   MQTT::drop\n                   MQTT::respond type CONNACK return_code 2\n                   MQTT::disconnect\n               }\n           }",
            return_value: "When called without an argument, this command returns the client identifier of MQTT message.",
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
            FormSpec { kind: FormKind::Default, synopsis: "MQTT::client_id (CLIENTID)?" },
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
