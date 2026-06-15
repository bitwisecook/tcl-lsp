//! `DIAMETER::length` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::length",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets diameter message length.",
            synopsis: &["DIAMETER::length"],
            snippet: "This iRule command returns the length of the current message,\nincluding the message header.\n\nThe value returned reflects the current length of the message at the\ninstant the iRule command is executed: if you store the length of a\nmessage in a variable and then modify the message, your stored length\nmay be incorrect.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__length.html",
            examples: "when DIAMETER_INGRESS {\n    log local0. \"Received a Diameter message of [DIAMETER::length] bytes\"\n}",
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DIAMETER::length",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
