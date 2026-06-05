//! `CACHE::priority` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::priority",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Adds a priority to cached documents.",
            synopsis: &["CACHE::priority CACHE_PRIORITY"],
            snippet: "Assigns a priority to cached documents. The priority value can be\nbetween 1 and 10 inclusive. This command allows users to designate\ndocuments that are costly to produce as being more important than\nothers to cache. This is particularly useful when you have a document\nthat is not requested often, but is expensive to produce (such as a\nserver-compressed document.) By increasing the priority, you are\nincreasing its likelihood of being served from the cache\n\nThe default priority value for entries in the cache is zero (0 = cache\npriority disabled).\n\nCACHE::priority <1 ..",
            source: "https://clouddocs.f5.com/api/irules/CACHE__priority.html",
            examples: "when HTTP_REQUEST {\n  switch -glob [HTTP::uri] {\n    \"*.zip\" -\n    \"*.tar\" -\n    \"*.gz\" {\n      # set the priority to 8 for this document if it's a compressed archive\n      CACHE::priority 8\n    }\n    \"*.gif\" -\n    \"*.jpg\" -\n    \"*.png\" {\n      # set the priority to 5 for this document if it's an image\n      CACHE::priority 5\n    }\n    \"/mustcache*\" {\n      # Any document matching /mustcache will be set to the highest priority.\n      CACHE::priority 10\n    }\n  }\n}",
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
