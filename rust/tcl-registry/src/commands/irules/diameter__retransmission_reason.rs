//! `DIAMETER::retransmission_reason` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::retransmission_reason",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the reason for retransmitting the current retransmitted request.",
            synopsis: &["DIAMETER::retransmission_reason"],
            snippet: "This iRule command returns the reason the current request was retransmitted.\nOtherwise, it returns 'none'.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__retransmission_reason.html",
            examples: "when MR_INGRESS {\n    if { [DIAMETER::is_retransmission] } {\n        log local0. \"reason [DIAMETER::retransmission_reason]\"\n        DIAMETER::persist reset\n        MR::message route pool /Common/alt_pool\n    }\n}",
            return_value: "'none', 'error_code', 'timeout' or 'irule'",
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
