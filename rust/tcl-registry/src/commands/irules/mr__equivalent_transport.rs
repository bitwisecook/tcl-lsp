//! `MR::equivalent_transport` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::equivalent_transport",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or sets the transport that is usable as an equivalent transport.",
            synopsis: &["MR::equivalent_transport", "MR::equivalent_transport none", "MR::equivalent_transport (('virtual' VIRTUAL_SERVER_OBJ) | ('config' TRANSPORT_CONFIG))"],
            snippet: "Gets or sets the transport that is usable as an equivalent transport. The equivalent transport may be used as an alternate when selecting a subsequent connection to the device the current connections communicates with.\n        \nGets the transport that is usable as an equivalent transport. The equivalent transport may be used as an alternate when selecting a subsequent connection to the device the current connections communicates with.\n            \nResets the transport that is usable as an equivalent transport.",
            source: "https://clouddocs.f5.com/api/irules/MR__equivalent_transport.html",
            examples: "when CLIENT_ACCEPTED {\n    MR::equivalent_transport config /Common/inbound_tc\n}",
            return_value: "Returns the current equivalent transport. This will contain the transport type and transport name. For example: 'config /Common/inbound_tc'.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MR::equivalent_transport" },
        ],
        ..CommandSpec::DEFAULT
    }
}
