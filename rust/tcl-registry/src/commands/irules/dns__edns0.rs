//! `DNS::edns0` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::edns0",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets (v11.0+) and sets (v11.1+) the values of the edns0 pseudo-RR.",
            synopsis: &[
                "DNS::edns0 'remove' ('nsid' | 'subnet')?",
                "DNS::edns0 'exists' ('nsid' | 'subnet')?",
                "DNS::edns0 'do' (BOOLEAN)?",
                "DNS::edns0 'sz' (UNSIGNED_SHORT)?",
            ],
            snippet: "This iRules command gets (v11.0+) and sets (v11.1+) the values of the\nedns0 pseudo-RR.\n\nNote: This command requires the DNS Profile, which is only enabled as\npart of GTM or the DNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__edns0.html",
            examples: "when DNS_REQUEST {\n  if { [DNS::edns0 exists] } {\n    log local0. [DNS::edns0 subnet address]\"\n  }\n}",
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DNS::edns0 'remove' ('nsid' | 'subnet')?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
