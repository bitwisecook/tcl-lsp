//! `DOSL7::enable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DOSL7::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enables blocking and detection of DoS attacks according to the ASM security policy configuration.",
            synopsis: &["DOSL7::enable (DOSL7_PROFILE_OBJ)?"],
            snippet: "Enables blocking and detection of DoS attacks according to the ASM\nsecurity policy configuration. When disabled using DOSL7::disable,\ntransactions will bypass DoS L7 for both detection and prevention.",
            source: "https://clouddocs.f5.com/api/irules/DOSL7__enable.html",
            examples: "when HTTP_REQUEST {\n    DOSL7::enable\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DOSL7::enable (DOSL7_PROFILE_OBJ)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::Dosl7State,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
