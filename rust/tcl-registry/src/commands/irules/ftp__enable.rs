//! `FTP::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FTP::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enable FTP protocol handler.",
            synopsis: &["FTP::enable"],
            snippet: "Enable FTP protocol handler for FTP message processing. This will enable detection of \"AUTH TLS/SSL\" for FTP.",
            source: "https://clouddocs.f5.com/api/irules/FTP__enable.html",
            examples: "when CLIENT_ACCEPTED {\n                if { !([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n                    FTP::enable\n                }\n            }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["CLIENT_ACCEPTED", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
