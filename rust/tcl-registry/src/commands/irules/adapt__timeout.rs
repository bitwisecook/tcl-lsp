//! `ADAPT::timeout` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::timeout",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets or returns the timeout attribute.",
            synopsis: &["ADAPT::timeout (ADAPT_CTX)? (ADAPT_SIDE)? (TIMEOUT_VALUE)?"],
            snippet: "The ADAPT::timeout command sets or returns the timeout attribute\nof the ADAPT filter on the current or specified side of the\nvirtual server connection for which the iRule is being executed.\nThe timeout (in milliseconds) is how long ADAPT will wait for\na result from the internal virtual server before deciding the\nservice is down.",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__timeout.html",
            examples: "when HTTP_RESPONSE {\n     if { [HTTP::header \"Content-Type\"] contains \"image\" } {\n        ADAPT::select ivs-icap-image\n        ADAPT::timeout 500\n     }\n     if { [HTTP::header \"Content-Type\"] contains \"video\" } {\n        ADAPT::select ivs-icap-video\n        ADAPT::timeout 2000\n     }\n }",
            return_value: "Returns the current or modified timeout in milliseconds.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP", "REQUESTADAPT", "RESPONSEADAPT"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ADAPT::timeout (ADAPT_CTX)? (ADAPT_SIDE)? (TIMEOUT_VALUE)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::IcapState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
