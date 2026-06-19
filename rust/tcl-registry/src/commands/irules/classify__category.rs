//! `CLASSIFY::category` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFY::category",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Allows to set or add a category name to the classification.",
            synopsis: &["CLASSIFY::category ('set' | 'add') CLASSIFY_CATEGORY_NAME"],
            snippet: "This command allows you to set or add a category name to the\nclassification.\n\n* Note: APM / AFM / PEM license is required for functionality to work.\n\nCLASSIFY::category set <category_name>\n\n     * will immediately classify flow as category_name. The classification\n       by the classification engine will be bypassed. Flow will have the unknown application classification token.\n\nCLASSIFY::category add <category_name>\n\n     * will add a category classification token to the final\n       classification result issued by the classification engine.",
            source: "https://clouddocs.f5.com/api/irules/CLASSIFY__category.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &["CLIENT_ACCEPTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "CLASSIFY::category ('set' | 'add') CLASSIFY_CATEGORY_NAME",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ClassificationState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
