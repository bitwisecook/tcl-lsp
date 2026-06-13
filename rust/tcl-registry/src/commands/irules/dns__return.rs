//! `DNS::return` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::return",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Skips all further processing after TCL execution and sends the DNS packet in the opposite direction.",
            synopsis: &["DNS::return"],
            snippet: "This iRules command skips all further processing after TCL execution\nand sends the DNS packet in the opposite direction.\nIn the DNS_REQUEST context, DNS::return signals that no further DNS\nresolution should occur for this request upon completion of the event.\nTo provide a useful response, resource record and header changes must\nbe made in the iRule. The next event triggered is the DNS_RESPONSE\nevent.\nIn the DNS_RESPONSE context, DNS::return sends a request back for\nadditional processing.\n\n**Important**: `DNS::return` does NOT stop iRule execution. You must\nfollow it with `return` to prevent the rest of the event handler\nfrom running.",
            source: "https://clouddocs.f5.com/api/irules/DNS__return.html",
            examples: "# Send one or more IP addresses for a response to an A query\n# Use on an LTM virtual server with a DNS profile enabled\nwhen DNS_REQUEST {\n        # Log query details\n        log local0. \"\\[DNS::question name\\]: [DNS::question name],\\\n                \\[DNS::question class\\]: [DNS::question class],\n                \\[DNS::question type\\]: [DNS::question type]\"\n\n        # Generate an answer with two A records",
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
            FormSpec { kind: FormKind::Default, synopsis: "DNS::return" },
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
