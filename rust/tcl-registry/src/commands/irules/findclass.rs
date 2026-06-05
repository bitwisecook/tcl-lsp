//! `findclass` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "findclass",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Searches a data group list for a member that starts with the specified string and returns the data-group member string.",
            synopsis: &["findclass STRING DATA_GROUP (SEPARATOR)?"],
            snippet: "Searches a data group list for a member whose key matches the specified\nstring, and if a match is found, returns the data-group member string.\n\nNote: findclass has been deprecated in v10 in favor of the new\nclass commands. The class command offers better functionality and\nperformance than findclass\nOnly the key value of the data group list member (the portion up to the\nfirst separator character, which defaults to space unless otherwise\nspecified) is compared to the specified string to determine a match.",
            source: "https://clouddocs.f5.com/api/irules/findclass.html",
            examples: "when HTTP_REQUEST {\n  set location [findclass [HTTP::uri] URIredirects_dg \" \"]\n  if { $location ne \"\" } {\n    HTTP::redirect $location\n  }\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
