//! `DNS::rdata` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::rdata",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or sets the resource record rdata field.",
            synopsis: &["DNS::rdata RR_OBJECT (VALUE)?"],
            snippet: "This iRules command gets or sets the resource record rdata field\n\nNote: This command requires the DNS Profile, which is only enabled as\npart of GTM or the DNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__rdata.html",
            examples: "when DNS_RESPONSE {\n         set rrs [DNS::answer]\n         foreach rr $rrs {\n             log local0. \"[DNS::rdata $rr]\"\n         }\n    }",
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
