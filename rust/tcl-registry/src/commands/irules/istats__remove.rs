//! `ISTATS::remove` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ISTATS::remove",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Removes the given iStat entirely.",
            synopsis: &["ISTATS::remove KEY"],
            snippet: "Removes the given iStat entirely.",
            source: "https://clouddocs.f5.com/api/irules/ISTATS__remove.html",
            examples: "ISTATS::remove \"ltm.pool /Common/mypool counter some_counter\"",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ISTATS::remove KEY",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::IStats,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Global,
        }],
        ..CommandSpec::DEFAULT
    }
}
