//! `peer` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "peer",
        traits: Traits::IS_SIDE_SWITCH,
        dialects: Some(DialectSet::IRULES),
        // `peer NESTING_SCRIPT` — unlike clientside/serverside, peer has
        // no bare query form, so the script body is required: exactly
        // one argument at index 0 (#501).
        arity: Arity::new(1, 1),
        // The nesting script (index 0) is a body evaluated under the
        // peer-side context; it runs synchronously in the caller's
        // frame, so the default `BodyKind::Plain` applies.
        arg_roles: &[(0, ArgRole::Body)],
        hover: Some(HoverSnippet {
            summary:
                "Causes the specified iRule commands to be evaluated under the peer-side context.",
            synopsis: &["peer ANY_CHARS"],
            snippet:
                "Causes the specified iRule commands to be evaluated under the peer-side context.",
            source: "https://clouddocs.f5.com/api/irules/peer.html",
            examples: "when SERVER_CONNECTED {\n  peer { TCP::collect }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "peer ANY_CHARS",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
