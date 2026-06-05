//! `ACCESS::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enables the access control enforcement for a particular request URI.",
            synopsis: &["ACCESS::enable"],
            snippet: "This command enables the access control enforcement for a particular\nrequest URI.\n\nACCESS::enable\n\n     * Enables the access control enforcement for a particular request\n       URI.\n\n * Requires APM module",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__enable.html",
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
        ..CommandSpec::DEFAULT
    }
}
