//! `UDP::debug_queue` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::debug_queue",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command can be used to enable/disable printing debug messages when UDP::max_rate iRule is in use.",
            synopsis: &["UDP::debug_queue BOOL_VALUE"],
            snippet: "UDP::debug_queue enable starts printing debug messages related to UDP::max_rate.\nUDP::debug_queue disable stops printing debug messages related to UDP::max_rate.",
            source: "https://clouddocs.f5.com/api/irules/UDP__debug_queue.html",
            examples: "when SERVER_CONNECTED {\n    # Set the rate to 1Mbps (125,000 bytes per second)\n    log local0. \"UDP set max rate: [UDP::max_rate 125000]\"\n    log local0. \"UDP get max rate: [UDP::max_rate]\"\n    # Enable printing debug messages.\n    log local0. \"Enable debugging [UDP::debug_queue enable]\"\n}",
            return_value: "None.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "UDP::debug_queue BOOL_VALUE" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::UdpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
