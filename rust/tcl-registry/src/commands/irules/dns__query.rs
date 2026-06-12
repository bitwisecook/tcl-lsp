//! `DNS::query` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::query",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or constructs and sends a query to the DNS-Express database for a name and type.",
            synopsis: &["DNS::query ('dnsx' | 'dns-express') NAME DNS_TYPE (DNSSEC)?"],
            snippet: "This iRules command returns or constructs and sends a query to the\nDNS-Express database for a name and type (IN class only).\n\nNote: This command requires the DNS Profile, which is only enabled as\npart of GTM or the DNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__query.html",
            examples: "when DNS_RESPONSE {\n        set rrsl [DNS::query dnsx nameserver.org SOA]\n        foreach rrs $rrsl {\n            foreach rr $rrs {\n                if { [DNS::type $rr] == \"SOA\" } {\n                    DNS::additional insert $rr\n                }\n            }\n        }\n    }",
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
            FormSpec { kind: FormKind::Default, synopsis: "DNS::query ('dnsx' | 'dns-express') NAME DNS_TYPE (DNSSEC)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::DnsState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
