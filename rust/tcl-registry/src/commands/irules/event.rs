//! `event` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "event",
        traits: Traits::DIAGRAM_ACTION,
        dialects: None,
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enables or disables evaluation of the specified iRule event or all iRule events on this connection.",
            synopsis: &[
                "event info",
                "event (enable | disable) ('all')?",
                "event EVENTNAME (enable | disable)",
            ],
            snippet: "Enables or disables evaluation of the specified iRule event, or all\niRule events, on this connection. However, the iRule continues to run.\n\n**Pattern — after drop/reject**: Always follow `drop` or `reject`\nwith `event disable all` and `return` to prevent other iRules from\nrunning on the now-invalid connection.",
            source: "https://clouddocs.f5.com/api/irules/event.html",
            examples: "when HTTP_RESPONSE {\n  COMPRESS::method prefer gzip\n  event disable\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "event info",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
