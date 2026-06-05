//! `PROFILE::fastL4` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::fastL4",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the value of a Fast L4 profile setting.",
            synopsis: &["PROFILE::fastL4 ATTR"],
            snippet: "Returns the current value of the specified setting in the assigned Fast L4 profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__fastL4.html",
            examples: "",
            return_value: "Returns the current value of the specified setting in the assigned Fast L4 profile.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "PROFILE::fastL4 ATTR" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::BigipConfig,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
