//! `listen` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "listen",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets up a related ephemeral listener to allow an incoming related connection to be established.",
            synopsis: &["listen (<'proto' UNSIGNED_SHORT> |"],
            snippet: "Sets up a related ephemeral listener to allow an incoming related\nconnection to be established. The source address and/or port of the\nrelated connection is unknown but the destination address and port are\nknown.",
            source: "https://clouddocs.f5.com/api/irules/listen.html",
            examples: "when RULE_INIT {\n      set my_port \"\"\n   }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["PERSIST_DOWN"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "listen (<'proto' UNSIGNED_SHORT> |" },
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
