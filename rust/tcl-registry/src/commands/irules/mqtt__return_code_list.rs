//! `MQTT::return_code_list` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::return_code_list",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get return-code-list of MQTT SUBACK message.",
            synopsis: &["MQTT::return_code_list"],
            snippet: "This command can be used to get return-code-list of MQTT message.\nNote that this command does not support a 'set' operation.\nIn order to change the retun-code-list please use 'MQTT::replace type SUBACK'.\nThis command is valid only for following MQTT message types:\n\n    SUBACK",
            source: "https://clouddocs.f5.com/api/irules/MQTT__return_code_list.html",
            examples: "when SERVER_CONNECTED {\n   set suback_count 0\n   set rclist [list]\n}",
            return_value: "When called without an argument, this command returns the return-code-list of MQTT SUBACK message.",
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
            FormSpec { kind: FormKind::Default, synopsis: "MQTT::return_code_list" },
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
