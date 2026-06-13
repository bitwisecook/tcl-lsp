//! `MR::available_for_routing` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::available_for_routing",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or sets the available_for_routing mode for the current connection.",
            synopsis: &["MR::available_for_routing (BOOLEAN)?"],
            snippet: "The MR::available_for_routing command sets or resets the available_for_routing mode of the current connection. If available_for_routing mode is enabled (upon completion of CLIENT_ACCEPTED event), the connection will be stored in the internal table of existing connections used for routing messages. This will make the connection available to have request messages routed towards it. If available_for_routing mode is disabled (upon completion of CLIENT_ACCEPTED event), the current connection will not be added to the internal table of existing connections.",
            source: "https://clouddocs.f5.com/api/irules/MR__available_for_routing.html",
            examples: "when CLIENT_ACCEPTED {\n                MR::available_for_routing no\n            }",
            return_value: "Returns the current value of the available_for_routing flag. This will be 'true' or 'false'.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MR::available_for_routing (BOOLEAN)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::MessageState,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
