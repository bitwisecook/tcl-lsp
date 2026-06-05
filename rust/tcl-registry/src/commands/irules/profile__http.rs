//! `PROFILE::http` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::http",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the value of an HTTP profile setting.",
            synopsis: &["PROFILE::http ATTR"],
            snippet: "Returns the current value of the specified setting in the assigned HTTP profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__http.html",
            examples: "# For examples of the command output, add a simple logging iRule to a VIP:\nwhen HTTP_REQUEST {\n   log local0. \"\\[PROFILE::http name\\]: [PROFILE::http name]\"\n}",
            return_value: "Returns the current value of the specified setting in the assigned HTTP profile.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "PROFILE::http ATTR" },
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
