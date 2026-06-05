//! `MQTT::topic` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::topic",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Manipulate topic(s) of MQTT message.",
            synopsis: &["MQTT::topic ?subcommand? ?args?", "MQTT::topic replace <new_topic> ?index?", "MQTT::topic count"],
            snippet: "This command can be used to manipulate topic(s) of MQTT message.\nThis command is valid only for following MQTT message types:\n\n    PUBLISH\n    SUBSCRIBE\n    UNSUBSCRIBE",
            source: "https://clouddocs.f5.com/api/irules/MQTT__topic.html",
            examples: "when MQTT_SERVER_INGRESS {\n    set smtype [MQTT::type]\n    if {$smtype == \"SUBACK\"} {\n       set mid [MQTT::packet_id]\n       set tc [table lookup -subtable \"packetid_count_table\" \"[client_addr]_[client_port]_${mid}\"]\n       set return_codes [MQTT::return_code_list]\n       set return_codes [lreplace $return_codes $tc $tc]\n       MQTT::replace type SUBACK packet_id $mid return_code_list $return_codes\n    }\n}",
            return_value: "When called without an argument, this command returns the topic-name of MQTT PUBLISH message, or the topic-name of the first topic of MQTT SUBSCRIBE and UNSUBSCRIBE messages.",
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
            FormSpec { kind: FormKind::Default, synopsis: "MQTT::topic ?subcommand? ?args?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
