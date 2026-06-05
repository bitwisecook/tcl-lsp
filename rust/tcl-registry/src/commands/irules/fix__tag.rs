//! `FIX::tag` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FIX::tag",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Defines/deletes the mapping between senderCompID and a tag map data group.",
            synopsis: &["FIX::tag map set SENDER DATA_GROUP", "FIX::tag map delete", "FIX::tag get TAG"],
            snippet: "This command can either retrieve tag value or update the mapping\nbetween senderCompID and a tag map data group. In latter case If a\nmapping is already defined in the profile attributes for\nsender-tag-map, it is overwritten by the iRule mapping.",
            source: "https://clouddocs.f5.com/api/irules/FIX__tag.html",
            examples: "when RULE_INIT {\n  # with the follow command, tag 10001 is replaced to 20001 for the messages sent by client_1\n  # before sending to pool member and reverse-replaced(20001 to 10001) to client_1\n  FIX::tag map set client_1 data_group_1\n  FIX::tag map set client_2 data_group_1\n  FIX::tag map set client_3 data_group_2\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FIX"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "FIX::tag map set SENDER DATA_GROUP" },
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
