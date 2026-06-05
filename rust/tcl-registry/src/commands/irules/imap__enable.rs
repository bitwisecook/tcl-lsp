//! `IMAP::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IMAP::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enable IMAP protocol handler.",
            synopsis: &["IMAP::enable"],
            snippet: "Enable IMAP protocol handler for IMAP message processing. This will enable detection of STARTTLS for IMAP.",
            source: "https://clouddocs.f5.com/api/irules/IMAP__enable.html",
            examples: "when CLIENT_ACCEPTED {\n                if { !([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n                    IMAP::enable\n                }\n            }",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "IMAP::enable" },
        ],
        ..CommandSpec::DEFAULT
    }
}
