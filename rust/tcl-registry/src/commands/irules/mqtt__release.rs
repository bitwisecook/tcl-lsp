//! `MQTT::release` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::release",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Releases the data collected via MQTT::collect iRule command",
            synopsis: &["MQTT::release"],
            snippet: "Releases the payload data collected via MQTT::collect iRule command for further processing.\n\nThis command is valid only when MQTT::collect has been called.",
            source: "https://clouddocs.f5.com/api/irules/MQTT__release.html",
            examples: "when MQTT_CLIENT_DATA {\n   set type [MQTT::type]\n   switch $type {\n       \"PUBLISH\" {\n          set payload [MQTT::payload]\n          MQTT::release\n          set found [class match $payload contains blacklisted_keywords_datagroup]\n          if { $found != \"\" } {\n              MQTT::disconnect\n          }\n       }\n   }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MQTT::release" },
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
