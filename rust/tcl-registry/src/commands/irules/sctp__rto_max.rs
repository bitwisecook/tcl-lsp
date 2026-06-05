//! `SCTP::rto_max` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::rto_max",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the maximum value of SCTP retransmission timeout.",
            synopsis: &["SCTP::rto_max (clientside | serverside)?"],
            snippet: "Returns the maximum value of SCTP retranmission timeout. Can specify the value on clientside or serverside.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__rto_max.html",
            examples: "when SERVER_CONNECTED {\n        log local0.info \"SCTP retransmission timeout maximum value is [SCTP::rto_max]\"\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SCTP::rto_max (clientside | serverside)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
