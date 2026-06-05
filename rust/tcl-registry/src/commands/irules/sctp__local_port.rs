//! `SCTP::local_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::local_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the local SCTP port/service number.",
            synopsis: &["SCTP::local_port (clientside | serverside)?"],
            snippet: "Returns the local SCTP port/service number. Can specify the port value on clientside or serverside.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__local_port.html",
            examples: "when CLIENT_ACCEPTED {\n        SCTP::collect\n        log local0.info \"Sctp local port is [SCTP::local_port]\"\n        log local0.info \"Sctp client port is [SCTP::client_port]\"\n        log local0.info \"Sctp mss is [SCTP::mss]\"\n        log local0.info \"sctp ppi is [SCTP::ppi]\"\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
