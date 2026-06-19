//! `ISESSION::deduplication` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ISESSION::deduplication",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Allows selection of deduplication based on L7 content inspection.",
            synopsis: &["ISESSION::deduplication BOOL_VALUE"],
            snippet: "Allows selection of deduplication based on L7 content inspection",
            source: "https://clouddocs.f5.com/api/irules/ISESSION__deduplication.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ISESSION::deduplication BOOL_VALUE",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
