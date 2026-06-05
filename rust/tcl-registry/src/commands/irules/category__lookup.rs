//! `CATEGORY::lookup` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CATEGORY::lookup",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get category of URL.",
            synopsis: &["CATEGORY::lookup URL ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')? ('-ip' IP)? ('-custom_cat_match' ANY_CHARS)?"],
            snippet: "This command returns the category of the supplied URL. (requires SWG license)\nThe URL needs to be of the form:\nscheme://domain:port/path?query_string#fragment_id\n\nThe query_string and fragment_id are optional. The entire list of categories supported is available in the UI under \"Secure Web Gateway\" in the APM section. Examples of categories include Sports, Shopping, etc. The response is a list of category names in a TCL array. Most input URLs result in a single category but some will return more than one.",
            source: "https://clouddocs.f5.com/api/irules/CATEGORY__lookup.html",
            examples: "when HTTP_REQUEST {\n        set this_uri http://[HTTP::host][HTTP::uri]\n        set reply [CATEGORY::lookup $this_uri]\n        log local0. \"Category lookup for $this_uri give $reply\"\n    }",
            return_value: "Returns a list of categories returned by the categorization engine depending on the category type specified (custom, request_default, or request_default_and_custom). If no type is specified, it will return request_default.",
        }),
        ..CommandSpec::DEFAULT
    }
}
