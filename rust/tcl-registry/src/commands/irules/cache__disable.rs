//! `CACHE::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Disables caching for this request.",
            synopsis: &["CACHE::disable"],
            snippet: "Disables caching for this request.\n\nCACHE::disable\n\n     * Disables caching for this request.",
            source: "https://clouddocs.f5.com/api/irules/CACHE__disable.html",
            examples: "when HTTP_REQUEST {\n  # Disable caching if the URI does not contain the string \"images\"\n  if { not ([HTTP::uri] contains \"images\") } {\n    CACHE::disable\n  }\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "CACHE::disable" },
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
