//! `HTTP::passthrough_reason` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::passthrough_reason",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the reason for the most recent switch to pass-through mode by the HTTP filter.",
            synopsis: &["HTTP::passthrough_reason ('as_num')?"],
            snippet: "This command returns the reason for the most recent switch to\npass-through mode by the HTTP filter.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__passthrough_reason.html",
            examples: "when HTTP_DISABLED { log local2. [HTTP::passthrough_reason] }",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "HTTP::passthrough_reason ('as_num')?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::HttpHeader,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
