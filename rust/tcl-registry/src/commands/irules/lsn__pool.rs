//! `LSN::pool` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::pool",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Explicitly set the LSN pool used for translation.",
            synopsis: &["LSN::pool LSN_POOL"],
            snippet: "Explicitly set the LSN pool used for translation.\n\nLSN::pool <pool_name>",
            source: "https://clouddocs.f5.com/api/irules/LSN__pool.html",
            examples: "",
            return_value: "LSN::pool",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP", "MR", "RTSP", "SIP"],
            also_in: &[
                "CLIENT_ACCEPTED",
                "CLIENT_DATA",
                "LB_FAILED",
                "LB_SELECTED",
                "SA_PICKED",
            ],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "LSN::pool LSN_POOL",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
