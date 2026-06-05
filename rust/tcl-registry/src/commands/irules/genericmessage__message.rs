//! `GENERICMESSAGE::message` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GENERICMESSAGE::message",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets values for messages in the generic message profile.",
            synopsis: &["GENERICMESSAGE::message (len | length)", "GENERICMESSAGE::message (src | source | dst | dest | destination) (SRC_DST)?", "GENERICMESSAGE::message is_request (BOOLEAN)?", "GENERICMESSAGE::message data (DATA)?"],
            snippet: "The GENERICMESSAGE::message command returns or sets values from\nthe current message being processed by the generic message profile.",
            source: "https://clouddocs.f5.com/api/irules/GENERICMESSAGE__message.html",
            examples: "when GENERICMESSAGE_INGRESS {\n    GENERICMESSAGE::message src us\n    GENERICMESSAGE::message dst them\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["GENERICMSG", "MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "GENERICMESSAGE::message (len | length)" },
        ],
        ..CommandSpec::DEFAULT
    }
}
