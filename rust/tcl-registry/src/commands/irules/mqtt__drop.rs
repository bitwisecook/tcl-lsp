//! `MQTT::drop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::drop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Drop the current MQTT message.",
            synopsis: &["MQTT::drop"],
            snippet: "This command can be used to drop the current MQTT message. The MQTT message will not be forwarded to its destination.\nThis command is valid for all MQTT message types.",
            source: "https://clouddocs.f5.com/api/irules/MQTT__drop.html",
            examples: "#Enrich MQTT username with SSL client-certificate common name, reject unauthorized accesses:\nwhen CLIENT_ACCEPTED {\n    set cn \"\"\n}",
            return_value: "None.",
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
            FormSpec { kind: FormKind::Default, synopsis: "MQTT::drop" },
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
