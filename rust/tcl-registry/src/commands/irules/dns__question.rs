//! `DNS::question` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::question",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets (v11.0+) or sets (v11.1+) the question field value.",
            synopsis: &[
                "DNS::question ('name' | 'type') (VALUE)?",
                "DNS::question 'class' (DNS_CLASS)?",
            ],
            snippet: "This iRules command gets (v11.0+) or sets (v11.1+) the question field\nvalue.\n\nNote: This command requires the DNS Profile, which is only enabled as\npart of GTM or the DNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__question.html",
            examples: "when DNS_REQUEST {\n    log local0. \"my question name: [DNS::question name]\"\n}",
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
            synopsis: "DNS::question ('name' | 'type') (VALUE)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
