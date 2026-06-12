//! `MQTT::collect` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::collect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Collect the specified amount of MQTT message payload data",
            synopsis: &["MQTT::collect (COLLECT)?"],
            snippet: "Collects the specified amount of MQTT message payload data before triggering a MQTT_CLIENT_DATA or MQTT_SERVER_DATA event.\n\nWhen collecting data in a clientside event, the MQTT_CLIENT_DATA event will be triggered.\nWhen collecting data in a serverside event, the MQTT_SERVER_DATA event will be triggered.\n\nThis command is valid only for following MQTT message types:\n\n    PUBLISH\n\nThis command allows you to perform various operations on MQTT PUBLISH message like modify its contents.\nNOTE: Please make sure that MQTT PUBLISH message expects to receive a payload by using [MQTT::payload length].",
            source: "https://clouddocs.f5.com/api/irules/MQTT__collect.html",
            examples: "when MQTT_CLIENT_DATA {\n   set type [MQTT::type]\n   switch $type {\n       \"PUBLISH\" {\n          set payload [MQTT::payload]\n          MQTT::release\n          set found [class match $payload contains blacklisted_keywords_datagroup]\n          if { $found != \"\" } {\n              MQTT::disconnect\n          }\n       }\n   }\n}",
            return_value: "",
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
            FormSpec { kind: FormKind::Default, synopsis: "MQTT::collect (COLLECT)?" },
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
