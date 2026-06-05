//! `sharedvar` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "sharedvar",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Allows a variable to be accessed in both sides of a VIP-targetting-VIP.",
            synopsis: &["sharedvar VARIABLE"],
            snippet: "Allows a variable to be accessed in both sides of a VIP-targetting-VIP",
            source: "https://clouddocs.f5.com/api/irules/sharedvar.html",
            examples: "when HTTP_RESPONSE {\nlog local0. \"vip1 @ response: private: $private\"\nlog local0. \"vip1 @ response: public: $public\"\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "sharedvar VARIABLE" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::Variable,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Global,
            },
        ],
        arg_roles: &[(0, ArgRole::VarWrite)],
        ..CommandSpec::DEFAULT
    }
}
