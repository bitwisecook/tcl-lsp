//! `SSE::field` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSE::field",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Basic access and manipulation of SSE message fields",
            synopsis: &["SSE::field ("],
            snippet: "This command provides the ability to create, read, update, and delete individual fields on a Server Sent Events (SSE) message.",
            source: "https://clouddocs.f5.com/api/irules/SSE__field.html",
            examples: "when SSE_RESPONSE {\n    log local0. \"event is \" [SSE::field get event]\n\n    SSE::field set my-custom-field  my-custom-value\n    log local0. \"my-custom-field is \" [SSE::field get my-custom-field]\n\n    SSE::field remove event\n}",
            return_value: "'get' returns the value of the specified field name. If field name does not exist, a null string is returned. 'set' and 'remove' do not return a value.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SSE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSE::field (" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
