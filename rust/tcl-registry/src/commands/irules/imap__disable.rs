//! `IMAP::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IMAP::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Disable IMAP protocol handler.",
            synopsis: &["IMAP::disable"],
            snippet: "Disable IMAP protocol handler for IMAP message processing. This will disable detection of STARTTLS for IMAP.",
            source: "https://clouddocs.f5.com/api/irules/IMAP__disable.html",
            examples: "when CLIENT_ACCEPTED {\n                if { ([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n                    IMAP::disable\n                }\n            }",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
