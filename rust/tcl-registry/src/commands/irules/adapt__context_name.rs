//! `ADAPT::context_name` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::context_name",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets the name of a dynamic adaptation context.",
            synopsis: &["ADAPT::context_name ADAPT_CTX"],
            snippet: "Obtains the name of an adaptation context. The name of a\ndynamic context was specified when it was created. The name\nof a static (profile) context is that of the ADAPT profile\non the side of the virtual server where the context resides.\n\nSyntax:\n\nADAPT::context_name <context>",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__context_name.html",
            examples: "when ADAPT_RESPONSE_RESULT {\n   set ctx [ADAPT::context_current]\n   set ctx_name [ADAPT::context_name $ctx]\n   log local0. \"ADAPT_RESPONSE_RESULT in context $ctx_name\"\n}",
            return_value: "Returns the context name.",
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
            synopsis: "ADAPT::context_name ADAPT_CTX",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::IcapState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
