//! `DNS::rr` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::rr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Creates a new resource record object with specified attributes or as a complete string.",
            synopsis: &["DNS::rr ANY_CHARS", "DNS::rr NAME DNS_TYPE DNS_CLASS TTL RDATA"],
            snippet: "This iRules command creates a new resource record object with specified\nattributes or as a complete string.\n\nNote: This command requires the DNS Profile, which is only enabled as\npart of GTM or the DNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__rr.html",
            examples: "when DNS_RESPONSE {\n        set rrs [DNS::answer]\n        foreach rr $rrs {\n            DNS::ttl $rr 1234\n        }\n        set new_rr [DNS::rr \"bigip3900-30.f5net.com. 88 IN A 1.2.3.4\"]\n        DNS::additional insert $new_rr\n    }",
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
            FormSpec { kind: FormKind::Default, synopsis: "DNS::rr ANY_CHARS" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::DnsState,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
