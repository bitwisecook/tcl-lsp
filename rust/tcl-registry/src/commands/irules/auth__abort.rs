//! `AUTH::abort` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::abort",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Cancels any outstanding auth operations in this authentication session.",
            synopsis: &["AUTH::abort AUTH_ID"],
            snippet: "Cancels any outstanding auth operations in this authentication session,\nand generates an AUTH_FAILURE event if there was an outstanding\nauthentication query in progress. This command invalidates the\nspecified authentication session ID, which should be discarded upon\ncalling this command.\n\nAUTH::abort authid\n\n     * Cancels any outstanding auth operations in this authentication\n       session, and generates an AUTH_FAILURE event if there was an\n       outstanding authentication query in progress.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__abort.html",
            examples: "when CLIENT_ACCEPTED {\n    set auth_http_successes 0\n    set auth_http_sufficient_successes 2\n}",
            return_value: "",
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
            FormSpec { kind: FormKind::Default, synopsis: "AUTH::abort AUTH_ID" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ApmState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
