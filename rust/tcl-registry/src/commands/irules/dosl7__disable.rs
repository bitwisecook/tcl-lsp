//! `DOSL7::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DOSL7::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Disables blocking and detection of DoS attacks according to the ASM security policy configuration.",
            synopsis: &["DOSL7::disable"],
            snippet: "Disables blocking and detection of DoS attacks according to the ASM\nsecurity policy configuration. When enabled using DOSL7::enable,\ntransactions will be enforced according to the DoS L7 ASM policy\nconfiguration for both detection and prevention.",
            source: "https://clouddocs.f5.com/api/irules/DOSL7__disable.html",
            examples: "when IN_DOSL7_ATTACK {\n    DOSL7::disable\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DOSL7::disable" },
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
