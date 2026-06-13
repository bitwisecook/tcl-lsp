//! `ADAPT::context_create` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::context_create",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Creates a new dynamic adaptation context.",
            synopsis: &["ADAPT::context_create (ADAPT_SIDE)? NAME"],
            snippet: "Creates a new dynamic adaptation context in the ADAPT filter on\nthe current or specified side of the virtual server connection\nfor which the iRule is being executed. Maybe called mulitple\ntimes to dynamically create chains of adaptation contexts.\n\nSyntax:\n\nADAPT::context_create <name>\n\n    * Creates a dynamic context on the current side.\n      This must be called from the request-adapt side, so has\n      the same effect as ADAPT::context_create request <name>.",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__context_create.html",
            examples: "when HTTP_RESPONSE {\n    # Configure a response context from the current (response) side.\n    ADAPT::select $rsp_ctx2 ivs-icap-rsp2\n    ADAPT::timeout $rsp_ctx2 2000\n}",
            return_value: "Returns the context handle.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP", "REQUESTADAPT"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ADAPT::context_create (ADAPT_SIDE)? NAME" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::IcapState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
