//! `CLASSIFY::application` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFY::application",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Allows to set or add an app name to the classification.",
            synopsis: &["CLASSIFY::application ('set' | 'add') CLASSIFY_APP_NAME"],
            snippet: "This command allows you to set or add an app name to the\nclassification.\n\n* Note: APM / AFM / PEM license is required for functionality to work.\n\nCLASSIFY::application set <app_name>\n\n     * will immediately classify flow as app_name. The classification by\n       the classification engine will be bypassed. The category classification token this app_name belongs to will also be attached.\n\nCLASSIFY::application add <app_name>\n\n     * adds an application classification token to the final\n       classification result issued by the classification engine.",
            source: "https://clouddocs.f5.com/api/irules/CLASSIFY__application.html",
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
            synopsis: "CLASSIFY::application ('set' | 'add') CLASSIFY_APP_NAME",
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
