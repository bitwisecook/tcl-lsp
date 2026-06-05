//! `CACHE::expire` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::expire",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Forces the document to be revalidated from the server.",
            synopsis: &["CACHE::expire"],
            snippet: "Forces the document to be revalidated from the server.\n\nCACHE::expire\n\n     * Forces the document to be revalidated from the server.",
            source: "https://clouddocs.f5.com/api/irules/CACHE__expire.html",
            examples: "when CACHE_REQUEST {\n  if { $expire equals 1 } {\n    CACHE::expire\n    log \"cache expire\"\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["CACHE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
