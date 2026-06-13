//! `DNS::enable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the service state to enabled for the current DNS packet.",
            synopsis: &["DNS::enable (DNS_COMPONENT)+"],
            snippet: "This iRules command sets the service state to enabled for the current\nDNS packet.\n\nNote: This command requires the DNS Profile, which is only enabled as\npart of GTM or the DNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__enable.html",
            examples: "ns express to resolve requests from a specific ip,\n            # disable dns express for all other requests\n            when DNS_REQUEST {\n                DNS::disable dnsx\n                if { [IP::client_addr] equals \"192.168.1.245\" } {\n                    DNS::enable dnsx\n                }\n            }",
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
            FormSpec { kind: FormKind::Default, synopsis: "DNS::enable (DNS_COMPONENT)+" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::DnsState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
