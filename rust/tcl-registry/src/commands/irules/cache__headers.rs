//! `CACHE::headers` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::headers",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the HTTP headers of the object in the cache.",
            synopsis: &["CACHE::headers"],
            snippet: "Returns the HTTP headers of the object in the cache.\nIf CACHE::header is used to manipulate the response headers prior to calling CACHE::headers, the modifications will not be reflected by CACHE::headers.\n\nCACHE::headers\n\n     * Returns the HTTP headers of the object in the cache as TCL Name / value pairs list.",
            source: "https://clouddocs.f5.com/api/irules/CACHE__headers.html",
            examples: "when CACHE_RESPONSE {\n  # log all  HTTP headers sent in cache response.\n  log local0. [CACHE::headers]\n}",
            return_value: "Returns the HTTP headers of the object in the cache as TCL Name / value pairs list.",
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
