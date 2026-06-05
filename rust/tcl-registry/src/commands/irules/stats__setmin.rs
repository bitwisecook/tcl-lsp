//! `STATS::setmin` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "STATS::setmin",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Ensures that the value of a Statistics profile setting is at the most value.",
            synopsis: &["STATS::setmin PROFILE_NAME FIELD_NAME (VALUE)?"],
            snippet: "Ensures that the value of the specified Statistics profile setting\n(field) is at the most value.",
            source: "https://clouddocs.f5.com/api/irules/STATS__setmin.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "STATS::setmin PROFILE_NAME FIELD_NAME (VALUE)?" },
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
