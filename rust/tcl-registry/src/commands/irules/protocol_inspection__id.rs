//! `PROTOCOL_INSPECTION::id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROTOCOL_INSPECTION::id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Provides protocol inspection match result.",
            synopsis: &["PROTOCOL_INSPECTION::id"],
            snippet: "This command provides inspection match result.",
            source: "https://clouddocs.f5.com/api/irules/PROTOCOL_INSPECTION__id.html",
            examples: "when PROTOCOL_INSPECTION_MATCH {\n    set id [PROTOCOL_INSPECTION::id]\n    log local0.debug \"inspection id: $id\"\n}",
            return_value: "PROTOCOL_INSPECTION::id returns inspection id array",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["PROTOCOL_INSPECTION"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "PROTOCOL_INSPECTION::id" },
        ],
        ..CommandSpec::DEFAULT
    }
}
