//! `CLASSIFICATION::category` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFICATION::category",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Deprecated: Provides classification category name.",
            synopsis: &["CLASSIFICATION::category"],
            snippet: "This command provides classification category name. (Example: mail,\ngaming)\n* Note: APM / AFM / PEM license is required for functionality to work.",
            source: "https://clouddocs.f5.com/api/irules/CLASSIFICATION__category.html",
            examples: "when CLASSIFICATION_DETECTED {\n  if { [CLASSIFICATION::category] equals \"chat\"}  {\n    drop\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["CLASSIFICATION"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "CLASSIFICATION::category" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ClassificationState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
