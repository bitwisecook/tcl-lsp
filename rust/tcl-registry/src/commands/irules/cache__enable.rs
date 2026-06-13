//! `CACHE::enable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Forces the document to be cached.",
            synopsis: &["CACHE::enable"],
            snippet: "Forces the document to be cached. You can also use this command to\ncache non-GET requests.\n\nNote: Should be used with extreme caution, as it allows caching of content marked private by server.\n\nCACHE::enable\n\n     * Forces the document to be cached.",
            source: "https://clouddocs.f5.com/api/irules/CACHE__enable.html",
            examples: "when HTTP_REQUEST {\n  if { [HTTP::uri] contains \"images\" } {\n    CACHE::enable\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "CACHE::enable" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::StreamProfile,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
