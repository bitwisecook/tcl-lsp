//! `MR::connection_mode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::connection_mode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the connection mode.",
            synopsis: &["MR::connection_mode"],
            snippet: "returns the connection mode of the current connection and the number of\nas configured in the peer object used to create the connection. Valid\nconnection modes as \"per-peer\", \"per-blade\", \"per-tmm\" or \"per-client\".\nFor incoming connections, it will return \"per-peer\".",
            source: "https://clouddocs.f5.com/api/irules/MR__connection_mode.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"[MR::connection_instance] [MR::connection_mode]\"\n}",
            return_value: "returns the connection mode",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MR::connection_mode" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::MessageState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
