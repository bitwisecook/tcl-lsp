//! `HTTP::is_redirect` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::is_redirect",
        traits: Traits::PURE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Returns a true value if the response is a redirect.",
            synopsis: &["HTTP::is_redirect"],
            snippet: "Returns a true value if the response is a redirect. Since only\nresponses can be redirects, it does not make sense to use this command\nin a clientside event.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__is_redirect.html",
            examples: "when HTTP_RESPONSE {\n  if { [HTTP::is_redirect] } {\n    log local0. \"Request redirected.\"\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Getter,
            synopsis: "HTTP::is_redirect",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpStatus,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
