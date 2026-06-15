//! `DNS::len` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::len",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the DNS packet message length.",
            synopsis: &["DNS::len"],
            snippet: "This iRules command returns the DNS packet message length.\n\nNote: This command requires the DNS Profile, which is only enabled as\npart of GTM or the DNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__len.html",
            examples: "ngth of each packet\n            when DNS_RESPONSE {\n                DNS::log \"PacketLength: \" [DNS::len]\n            }",
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
            synopsis: "DNS::len",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
