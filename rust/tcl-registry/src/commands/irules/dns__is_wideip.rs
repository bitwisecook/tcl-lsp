//! `DNS::is_wideip` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::is_wideip",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns status (true/false) if a string is a configured wide IP.",
            synopsis: &["DNS::is_wideip DNS_STRING"],
            snippet: "This iRules command returns status (true/false) if a string is a\nconfigured wide IP.\n\nNote: This command functions only in the context of LTM iRules and\nrequires the DNS Profile, which is only enabled as part of GTM or the\nDNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__is_wideip.html",
            examples: "when DNS_REQUEST {\n  if { [DNS::is_wideip [DNS::question name]] } {\n    log local0. \"[DNS::question name] is a wideIP\"\n  } else {\n      log local0. \"[DNS::question name] is not a wideIP\"\n  }\n}",
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
