//! `STATS::setmax` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "STATS::setmax",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Ensures that the value of a Statistics profile setting is at the least value.",
            synopsis: &["STATS::setmax PROFILE_NAME FIELD_NAME (VALUE)?"],
            snippet: "Ensures that the value of the specified Statistics profile setting\n(field) is at the least value.",
            source: "https://clouddocs.f5.com/api/irules/STATS__setmax.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "STATS::setmax PROFILE_NAME FIELD_NAME (VALUE)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::IStats,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
