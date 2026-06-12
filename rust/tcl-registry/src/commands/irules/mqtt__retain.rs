//! `MQTT::retain` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::retain",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get or set retain flag of MQTT PUBLISH message.",
            synopsis: &["MQTT::retain ('0' | '1')?"],
            snippet: "This command can be used to get or set retain flag of MQTT message.\nThis command is valid only for following MQTT message types:\n\n    PUBLISH",
            source: "https://clouddocs.f5.com/api/irules/MQTT__retain.html",
            examples: "# Convert PUBLISH for topics in a retain_datagroup to retain messages\nwhen MQTT_CLIENT_INGRESS {\n   set type [MQTT::type]\n   switch $type {\n      \"PUBLISH\" {\n          if {[MQTT::retain] eq 0} {\n              if { [class exists retain_datagroup] } {\n                  if {[class match [MQTT::topic] starts_with retain_datagroup]} {\n                     MQTT::retain 1\n                  }\n              }\n          }\n      }\n   }\n}",
            return_value: "When called without an argument, this command returns the retain flag of MQTT PUBLISH message.",
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
            FormSpec { kind: FormKind::Default, synopsis: "MQTT::retain ('0' | '1')?" },
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
