//! `CLASSIFICATION::username` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFICATION::username",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Provides username associated with classification results.",
            synopsis: &["CLASSIFICATION::username"],
            snippet: "CLASSIFICATION::username",
            source: "https://clouddocs.f5.com/api/irules/CLASSIFICATION__username.html",
            examples: "when CLASSIFICATION_DETECTED {\n        set res [CLASSIFICATION::username]\n        log local0.debug \"DPI username: $res\"\n    }",
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
            FormSpec { kind: FormKind::Default, synopsis: "CLASSIFICATION::username" },
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
