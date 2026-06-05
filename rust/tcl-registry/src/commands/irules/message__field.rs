//! `MESSAGE::field` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MESSAGE::field",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Various operations for a message's fields.",
            synopsis: &["MESSAGE::field ( ('names') |"],
            snippet: "This command is used for below mentioned operations for a message's field.\nThis is valid for messages of the following protocols:\n\n    SIP",
            source: "https://clouddocs.f5.com/api/irules/MESSAGE__field.html",
            examples: "when MR_INGRESS {\n    switch ( [MESSAGE::proto] ) {\n        \"SIP\" {\n           if { [MESSAGE::type] eq \"request\" } {\n              set uri [MESSAGE::field value ':uri']\n              log local0. \"Message's URI is : $uri\"\n           }\n        }\n    }\n}",
            return_value: "Returns value depends on the subcommands. See description for more details.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MESSAGE::field ( ('names') |" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::MessageState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
