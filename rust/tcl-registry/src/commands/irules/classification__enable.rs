//! `CLASSIFICATION::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFICATION::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Deprecated: Enables classification for the current flow.",
            synopsis: &["CLASSIFICATION::enable"],
            snippet: "This command enables classification for the current flow.\n\nCLASSIFICATION::enable",
            source: "https://clouddocs.f5.com/api/irules/CLASSIFICATION__enabled.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "CLASSIFICATION::enable" },
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
