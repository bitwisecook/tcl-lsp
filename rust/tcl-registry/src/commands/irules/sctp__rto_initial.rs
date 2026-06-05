//! `SCTP::rto_initial` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::rto_initial",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the initial value of SCTP retransmission timeout.",
            synopsis: &["SCTP::rto_initial (clientside | serverside)?"],
            snippet: "Returns the initial value of SCTP retranmission timeout. Can specify the value on clientside or serverside.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__rto_initial.html",
            examples: "when CLIENT_ACCEPTED {\n        log local0.info \"SCTP retransmission timeout initial value is [SCTP::rto_initial]\"\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
