//! `DIAMETER::disconnect` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::disconnect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sends Disconnect-Peer-Request to client or server based on context.",
            synopsis: &["DIAMETER::disconnect ORIGIN_HOST ORIGIN_REALM DIAMETER_DISCONNECT_CAUSE"],
            snippet: "This iRule command sends Disconnect-Peer-Request to the client (if run\non clientside) or to the server (if run on serverside).",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__disconnect.html",
            examples: "when DIAMETER_INGRESS {\n    # 2 = DO_NOT_WANT_TO_TALK_TO_YOU (RFC 6733 sec 5.4.3)\n    DIAMETER::disconnect \"bigip.core.example.com\" \"example.com\" 2\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "DIAMETER::disconnect ORIGIN_HOST ORIGIN_REALM DIAMETER_DISCONNECT_CAUSE" },
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
