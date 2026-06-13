//! `DNS::type` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or sets the resource record type field.",
            synopsis: &["DNS::type RR_OBJECT (DNS_TYPE)?"],
            snippet: "This iRules command gets or sets the resource record type field\n\nNote: This command functions only in the context of LTM iRules and\nrequires the DNS Profile, which is only enabled as part of GTM or the\nDNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__type.html",
            examples: "when DNS_RESPONSE {\n        set rrs [DNS::answer]\n        foreach rr $rrs {\n            if { [DNS::type $rr] == \"SOA\" } {\n                DNS::answer remove $rr\n            }\n        }\n    }",
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
            FormSpec { kind: FormKind::Default, synopsis: "DNS::type RR_OBJECT (DNS_TYPE)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::DnsState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
