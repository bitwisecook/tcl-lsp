//! `GTP::discard` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::discard",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Discards the current message.",
            synopsis: &["GTP::discard"],
            snippet: "Discards the current message",
            source: "https://clouddocs.f5.com/api/irules/GTP__discard.html",
            examples: "when GTP_SIGNALLING_INGRESS {\n    GTP::discard\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "GTP::discard",
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
