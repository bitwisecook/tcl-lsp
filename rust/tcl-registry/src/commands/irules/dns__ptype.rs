//! `DNS::ptype` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::ptype",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the type of the DNS packet.",
            synopsis: &["DNS::ptype"],
            snippet: "This iRules command returns the type of the DNS packet.\n\nNote: This command requires the DNS Profile, which is only enabled as\npart of GTM or the DNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__ptype.html",
            examples: "OMAIN response is going to be sent,\n            # instead attach a record to resolve to.\n            when DNS_RESPONSE {\n                if { [DNS::ptype] == \"NXDOMAIN\" } {\n                    DNS::header rcode NOERROR\n                    DNS::answer insert \"[DNS::question name]. 60 [DNS::question class] [DNS::question type] 192.168.1.245\"\n                }\n            }",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DNS::ptype" },
        ],
        ..CommandSpec::DEFAULT
    }
}
