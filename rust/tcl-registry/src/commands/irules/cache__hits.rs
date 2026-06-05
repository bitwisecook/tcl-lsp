//! `CACHE::hits` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::hits",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the document cache hits.",
            synopsis: &["CACHE::hits"],
            snippet: "Returns the document cache hits.\n\nCACHE::hits\n\n     * Returns the document cache hits.",
            source: "https://clouddocs.f5.com/api/irules/CACHE__hits.html",
            examples: "when CACHE_REQUEST {\n  log local0. \"[CACHE::hits] cache hits for document at [HTTP::uri]\"\n}",
            return_value: "Returns the document cache hits.",
        }),
        ..CommandSpec::DEFAULT
    }
}
