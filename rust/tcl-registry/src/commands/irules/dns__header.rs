//! `DNS::header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::header",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets (v11.0+) or sets (v11.1+) simple bits or byte fields.",
            synopsis: &["DNS::header <field> ?value?", "DNS::header id ?UNSIGNED_SHORT?", "DNS::header rcode ?RCODE_VALUE?"],
            snippet: "This iRules command gets or sets simple bits or byte fields. Read-only\nform introduced in v11.0, Read-write capability added in v11.1.\n\nNote: This command requires the DNS Profile, which is only enabled as\npart of GTM or the DNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__header.html",
            examples: "queries from a specific ip\n            when DNS_REQUEST {\n                if { [IP::client_addr] equals \"192.168.1.245\" } {\n                    DNS::answer clear\n                    DNS::header rcode REFUSED\n                    DNS::return\n                    return\n                }\n            }",
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
            FormSpec { kind: FormKind::Default, synopsis: "DNS::header <field> ?value?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
