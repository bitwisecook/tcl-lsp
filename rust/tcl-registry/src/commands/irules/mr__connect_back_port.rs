//! `MR::connect_back_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::connect_back_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or sets connect_back_port for the current connection.",
            synopsis: &["MR::connect_back_port (NONNEGATIVE_INTEGER)?"],
            snippet: "The MR::connect_back_port command gets or sets connect_back_port for the current connection, which is used to enable bi-directional persistence if the client connected through an ephemeral port.",
            source: "https://clouddocs.f5.com/api/irules/MR__connect_back_port.html",
            examples: "when CLIENT_ACCEPTED {\n                MR::connect_back_port 5678\n            }",
            return_value: "Returns current connect_back_port value.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MR::connect_back_port (NONNEGATIVE_INTEGER)?" },
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
