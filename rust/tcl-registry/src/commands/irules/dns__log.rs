//! `DNS::log` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::log",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Controls log publisher in DNS log profile.",
            synopsis: &["DNS::log (MESSAGE)?"],
            snippet: "There are two version of this command.  DNS::log by itself returns a boolean indicating whether a DNS Logging Profile is configured in the DNS profile.  DNS::log with an argument logs a message to that log publisher.",
            source: "https://clouddocs.f5.com/api/irules/DNS__log.html",
            examples: "# Send one or more IP addresses for a response to an A query\n            # Use on an LTM virtual server with a DNS profile enabled\n            when DNS_REQUEST {\n                # Log query details\n                DNS::log \"DNS question name: [DNS::question name],\n                    DNS question class: [DNS::question class],\n                    DNS question type: [DNS::question type]\"\n\n                # Generate an answer with two A records",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DNS"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
