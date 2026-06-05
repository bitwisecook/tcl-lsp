//! `MQTT::disconnect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::disconnect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Disconnect the MQTT connection.",
            synopsis: &["MQTT::disconnect"],
            snippet: "This command disconnects the MQTT connection.",
            source: "https://clouddocs.f5.com/api/irules/MQTT__disconnect.html",
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
            FormSpec { kind: FormKind::Default, synopsis: "MQTT::disconnect" },
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
