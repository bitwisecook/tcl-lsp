//! `CATEGORY::safesearch` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CATEGORY::safesearch",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get safe search key and value pairs.",
            synopsis: &["CATEGORY::safesearch URL ('-ip' IP)?"],
            snippet: "Checks for safe search parameters for the given URL, returns them in list form with the first entry being the key, and the second being the value. Repeated in list for multiple results. (requires SWG license)",
            source: "https://clouddocs.f5.com/api/irules/CATEGORY__safesearch.html",
            examples: "when HTTP_REQUEST {\n    set this_uri http://[HTTP::host][HTTP::uri]\n    set reply [CATEGORY::safesearch $this_uri]\n    set len [llength $reply]\n    if { $len equals 2 } {\n        log local0. \"uri $this_uri returns safesearch key=[lindex $reply 0] and value=[lindex $reply 1]\"\n        if { not([HTTP::uri] contains \"&[lindex $reply 0]=[lindex $reply 1]\") } {\n            HTTP::uri [HTTP::uri]&[lindex $reply 0]=[lindex $reply 1]\n        }\n    }\n}",
            return_value: "Returns a list of alternating key and value pairs. E.g.: [key1, value1, key2, value2]",
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
        ..CommandSpec::DEFAULT
    }
}
