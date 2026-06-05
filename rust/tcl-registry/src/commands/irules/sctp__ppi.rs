//! `SCTP::ppi` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::ppi",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets the SCTP payload protocol indicator.",
            synopsis: &["SCTP::ppi (PPI_ID)?"],
            snippet: "Returns or sets the SCTP payload protocol indicator.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__ppi.html",
            examples: "when CLIENT_ACCEPTED {\n        SCTP::collect\n        log local0.info \"Sctp local port is [SCTP::local_port]\"\n        log local0.info \"Sctp client port is [SCTP::client_port]\"\n        log local0.info \"Sctp mss is [SCTP::mss]\"\n        log local0.info \"sctp ppi is [SCTP::ppi]\"\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
