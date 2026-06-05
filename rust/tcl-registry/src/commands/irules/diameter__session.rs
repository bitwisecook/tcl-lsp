//! `DIAMETER::session` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::session",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or sets the session-id attribute-value pair.",
            synopsis: &["DIAMETER::session (SESSION_ID)?"],
            snippet: "This iRule command gets or sets the value of session-id AVP (code 263)\nin the message.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__session.html",
            examples: "when DIAMETER_INGRESS {\n    log local0. \"Received a DIAMETER message for session [DIAMETER::session]\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER", "MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DIAMETER::session (SESSION_ID)?" },
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
