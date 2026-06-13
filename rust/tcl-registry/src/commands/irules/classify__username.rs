//! `CLASSIFY::username` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFY::username",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Assigns username to the flow.",
            synopsis: &["CLASSIFY::username USERNAME (CONTEXT)?"],
            snippet: "This command assigns username to the flow. It could be used for\nreporting / statistics / etc.",
            source: "https://clouddocs.f5.com/api/irules/CLASSIFY__username.html",
            examples: "when CLIENT_ACCEPTED {\n    CLASSIFY::username superuser\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "CLASSIFY::username USERNAME (CONTEXT)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ClassificationState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
