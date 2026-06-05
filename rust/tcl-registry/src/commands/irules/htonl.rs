//! `htonl` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "htonl",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Converts the unsigned integer from host byte order to network byte order.",
            synopsis: &["htonl NUMBER"],
            snippet: "Convert the unsigned integer from host byte order to network byte\norder.",
            source: "https://clouddocs.f5.com/api/irules/htonl.html",
            examples:
                "when HTTP_REQUEST {\n  set hostlong 12345678\n  set netlong [htonl $hostlong]\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
