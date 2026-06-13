//! `CLASSIFICATION::protocol` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFICATION::protocol",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Depreated: provides classification for the least explicit application name.",
            synopsis: &["CLASSIFICATION::protocol"],
            snippet: "This command provides classification for the least explicit application\nname. (Example: http, ssl)\n\n* Note: APM / AFM / PEM license is required for functionality to work.\n\nCLASSIFICATION::protocol",
            source: "https://clouddocs.f5.com/api/irules/CLASSIFICATION__protocol.html",
            examples: "when CLASSIFICATION_DETECTED {\n  if { [CLASSIFICATION::protocol] == \"http\"}  {\n    drop\n  }\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "CLASSIFICATION::protocol" },
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
