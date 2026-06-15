//! `MR::prime` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::prime",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "establishes an outgoing connection to the specified host or hosts using the specified transport",
            synopsis: &[
                "MR::prime (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG)) ((pool POOL_OBJ) | (host HOST)))?",
            ],
            snippet: "The MR::prime command instructs the Message Routing Framework to establish an outgoing connection to a specified host or pool if one does not exist. The setting of the specified virtual or transport-config will be used to establish the connection. If a pool is provided, outgoing connections will be created to all active poolmembers of the specified pool.",
            source: "https://clouddocs.f5.com/api/irules/MR__prime.html",
            examples: "when CLIENT_ACCEPTED {\n                MR::prime config /Common/my_tc pool /Common/default_pool\n            }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "MR::prime (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG)) ((pool POOL_OBJ) | (host HOST)))?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::MessageState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
