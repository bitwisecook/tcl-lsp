//! `DNS::name` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::name",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or sets the resource record name field.",
            synopsis: &["DNS::name RR_OBJECT (VALUE)?"],
            snippet: "This iRules command gets or sets the resource record name field.\n\nNote: This command requires the DNS Profile, which is only enabled as\npart of GTM or the DNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__name.html",
            examples: "s responses returned to a specific client ip\n            when DNS_RESPONSE {\n                if { [IP::client_addr] equals \"192.168.1.245\" } {\n                    DNS::log [DNS::name [DNS::answer]]\n                }\n            }",
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
            FormSpec { kind: FormKind::Default, synopsis: "DNS::name RR_OBJECT (VALUE)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
