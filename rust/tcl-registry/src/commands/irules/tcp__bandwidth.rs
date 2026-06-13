//! `TCP::bandwidth` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::bandwidth",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the estimated bandwidth of the connection.",
            synopsis: &["TCP::bandwidth"],
            snippet: "Returns the estimated bandwidth measured as \"congestion window\" / \"the measured round trip time\".\nThe values returned are only estimates, and can vary even during the connection.\n\nNote: Starting with BIG-IP v9.4.2, client bandwidth calculations are unavailable, always returning 0. Starting with BIG-IP v12.0 nonzero values are returned.",
            source: "https://clouddocs.f5.com/api/irules/TCP__bandwidth.html",
            examples: "when CLIENT_DATA {\nif {[HTTP::uri] starts_with \"/xxx.css\"}{\n      set bandwidth [TCP::bandwidth]\n      if {$bandwidth < XXX} {\n         HTTP::uri \"/boring-xxx.css\"\n      }\n   }\n}",
            return_value: "The estimated bandwidth in kilobits per second.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &[],
            also_in: &["SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::bandwidth" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
