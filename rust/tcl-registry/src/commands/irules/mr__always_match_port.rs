//! `MR::always_match_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::always_match_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or sets the always_match_port mode for the router.",
            synopsis: &["MR::always_match_port (BOOLEAN)?"],
            snippet: "The MR::always_match_port command sets or resets the always_match_port mode of the current router. If always_match_port mode is enabled (upon completion of CLIENT_ACCEPTED event), the router will only forward messages to existing connections where the remote port matches the remote port of the selected destination. If an existing connection is not found, a new connection will be created. Setting this mode will keep MRF from forwarding messages to incoming connections (since the incoming connection likely uses a ephemeral port as the source port).",
            source: "https://clouddocs.f5.com/api/irules/MR__always_match_port.html",
            examples: "when CLIENT_ACCEPTED {\n                MR::always_match_port no\n            }",
            return_value: "Returns the current value of the always_match_port flag. This will be 'true' or 'false'.",
        }),
        ..CommandSpec::DEFAULT
    }
}
