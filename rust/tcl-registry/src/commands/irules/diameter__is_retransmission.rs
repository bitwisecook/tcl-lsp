//! `DIAMETER::is_retransmission` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::is_retransmission",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns true if it is a retransmitted request, otherwise, returns false.",
            synopsis: &["DIAMETER::is_retransmission"],
            snippet: "This iRule command returns true if the current message is a retransmitted request.\nOtherwise, it returns false.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__is_retransmission.html",
            examples: "when MR_INGRESS {\n    if { [DIAMETER::is_retransmission] } {\n        DIAMETER::persist reset\n        MR::message route pool /Common/alt_pool\n    }\n}",
            return_value: "TRUE or FALSE",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER", "MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
