//! `CATEGORY::analytics` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CATEGORY::analytics",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Controls response analytics engine.",
            synopsis: &["CATEGORY::analytics BOOL_VALUE"],
            snippet: "Enables or disables the analytics server on a per request basis (requires SWG license)",
            source: "https://clouddocs.f5.com/api/irules/CATEGORY__analytics.html",
            examples: "when HTTP_REQUEST {\n    set this_uri http://[HTTP::host][HTTP::uri]\n    set reply [CATEGORY::lookup $this_uri]\n    log local0. \"uri $this_uri returns category=$reply\"\n    if { $reply equals \"Adult Material\" } {\n        CATEGORY::analytics enable\n    }\n}",
            return_value: "No return value",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["CATEGORY", "FASTHTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "CATEGORY::analytics BOOL_VALUE" },
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
