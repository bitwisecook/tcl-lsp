//! `HTTP::fallback` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::fallback",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Specifies or overrides a fallback host specified in the HTTP profile.",
            synopsis: &["HTTP::fallback <host>"],
            snippet: "Specifies or overrides the fallback host specified in the HTTP profile.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__fallback.html",
            examples: "when LB_FAILED {\n  HTTP::fallback \"http://siteunavailable.mysite.com/\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &["LB_FAILED", "MR_FAILED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Setter,
            synopsis: "HTTP::fallback <host>",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpHeader,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
