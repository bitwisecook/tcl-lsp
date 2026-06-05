//! `HTTP2::disconnect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::disconnect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command allows you to cleanly terminate the current HTTP/2 session.",
            synopsis: &["HTTP2::disconnect"],
            snippet: "Cleanly terminates the current HTTP/2 session, if HTTP/2 is active.",
            source: "https://clouddocs.f5.com/api/irules/HTTP2__disconnect.html",
            examples: "when HTTP_REQUEST {\n    if {[HTTP2::active]} {\n        HTTP2::disconnect\n    }\n}",
            return_value: "The return is 0 if the disconnect was clean. If a GOAWAY frame cannot be sent, an error will be returned.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "HTTP2::disconnect" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ConnectionControl,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
