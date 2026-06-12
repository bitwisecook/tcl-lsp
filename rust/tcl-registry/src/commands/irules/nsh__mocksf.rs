//! `NSH::mocksf` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "NSH::mocksf",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Set option to mock SF functionality for NSH.",
            synopsis: &["NSH::mocksf"],
            snippet: "Set option to mock SF functionality for NSH.",
            source: "https://clouddocs.f5.com/api/irules/NSH__mocksf.html",
            examples: "cksf option for NSH.\n            when FLOW_INIT {\n                NSH::mocksf\n            }",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "NSH::mocksf" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
