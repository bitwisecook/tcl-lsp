//! `ADAPT::context_current` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::context_current",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets the current context.",
            synopsis: &["ADAPT::context_current"],
            snippet: "Obtains a handle for the current context. The current context\nis usually that in which the event occurred from which this\ncommand was issued.\n\nSyntax:\n\nADAPT::context_current",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__context_current.html",
            examples: "when ADAPT_REQUEST_RESULT {\n    set ctx [ADAPT::context_current]\n    if {$ctx == $req_ctx2 && $need_another_ctx} {\n        set req_ctx3 [ADAPT::context_create my_req_ctx3]\n        ADAPT::select $req_ctx3 ivs-icap-req3\n    }\n}",
            return_value: "Returns the handle of the current context.",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ADAPT::context_current" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::IcapState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
