//! `PROFILE::serverssl` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::serverssl",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the value of a Server SSL profile setting.",
            synopsis: &["PROFILE::serverssl ATTR"],
            snippet: "Returns the current value of the specified setting in the assigned Server SSL profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__serverssl.html",
            examples: "when HTTP_REQUEST {\n    if {[PROFILE::exists serverssl] == 1}{\n        log local0. \"server SSL profile enabled: [PROFILE::serverssl name]\"\n    }\n}",
            return_value: "Returns the current value of the specified setting in the assigned Server SSL profile.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "PROFILE::serverssl ATTR" },
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
