//! `PROFILE::fasthttp` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::fasthttp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the value of a Fast HTTP profile setting.",
            synopsis: &["PROFILE::fasthttp ATTR"],
            snippet: "Returns the current value of the specified setting in the assigned Fast HTTP profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__fasthttp.html",
            examples: "",
            return_value: "Returns the current value of the specified setting in the assigned Fast HTTP profile.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "PROFILE::fasthttp ATTR" },
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
