//! `LB::detach` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::detach",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Detaches the server-side connection from the client-side.",
            synopsis: &["LB::detach ('disable')?"],
            snippet: "This command detaches the server-side connection from the client-side.\n\nLB::detach\n    Detaches the server side connection from the client-side connection.\n    Use in conjunction with TCP::notify as best practice.",
            source: "https://clouddocs.f5.com/api/irules/LB__detach.html",
            examples: "when USER_RESPONSE {\n    LB::detach\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "LB::detach ('disable')?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::PoolSelection,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Server,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
