//! `SSL::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::disable",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Disables SSL processing.",
            synopsis: &["SSL::disable (clientside | serverside)?"],
            snippet: "Disables SSL processing. This command is useful when using a virtual server that services both SSL and non-SSL traffic, or when you want to selectively re-encrypt traffic to pool members.\n\nNote: Disabling SSL on the serverside only applies before serverside connection has been established (SERVER_CONNECTED) or when the clientside of the connection is in a detached state (e.g., oneconnect, LB::detach).",
            source: "https://clouddocs.f5.com/api/irules/SSL__disable.html",
            examples: "when SERVER_CONNECTED {\n    if { $usessl == 0 } {\n        SSL::disable\n    }\n}",
            return_value: "SSL::disable [clientside | serverside] Disables SSL processing on one side of the LTM. Sends an SSL alert to the peer requesting termination of SSL processing.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSL::disable (clientside | serverside)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::SslState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
