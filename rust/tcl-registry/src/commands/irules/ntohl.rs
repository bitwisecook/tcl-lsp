//! `ntohl` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ntohl",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Converts the unsigned integer from network byte order to host byte order.",
            synopsis: &["ntohl NUMBER"],
            snippet: "Convert the unsigned integer from network byte order to host byte\norder.",
            source: "https://clouddocs.f5.com/api/irules/ntohl.html",
            examples:
                "when HTTP_REQUEST{\n  set netlong 12345678\n  set hostlong [ntohl $netlong]\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
