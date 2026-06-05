//! `ACCESS::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Control enforcement for a particular request URI.",
            synopsis: &["ACCESS::disable"],
            snippet: "This command disables the access control enforcement for a particular\nrequest URI. The request is passed through access control module\nwithout any access control checks (excludes valid session check as well\nas policy allowed check).\n\nACCESS::disable\n\n     * Disable the access control enforcement for a particular request\n       URI.\n\n * Requires APM module",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__disable.html",
            examples: "when HTTP_REQUEST {\n\n       # Check the requested HTTP path\n       switch -glob [string tolower [HTTP::path]] {\n              \"/apm_uri1*\" -\n              \"/apm_uri2*\" -\n              \"/apm_uri3*\" {\n                     # Enable APM for these paths\n                     ACCESS::enable\n              }\n              default {\n                     # Disable APM for all other paths\n                     ACCESS::disable\n              }\n       }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ACCESS::disable" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ApmState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
