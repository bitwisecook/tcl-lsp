//! `PROFILE::persist` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::persist",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the value of a persistence profile setting.",
            synopsis: &["PROFILE::persist ((instance PROFILE_PERSIST ATTR) | (mode MODE ATTR))"],
            snippet: "Returns the current value of the specified setting in the assigned persistence profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__persist.html",
            examples: "",
            return_value: "Returns the current value of the specified setting in the assigned persistence profile.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "PROFILE::persist ((instance PROFILE_PERSIST ATTR) | (mode MODE ATTR))" },
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
