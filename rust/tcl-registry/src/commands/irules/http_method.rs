//! `http_method` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http_method",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the action of the HTTP request.",
            synopsis: &["http_method"],
            snippet: "Returns the action of the HTTP request. Common values are GET and\nPOST. This command is a BIG-IP version 4.X variable, provided for\nbackward-compatibility. You can use the equivalent 9.Xcommand\nHTTP::method instead.",
            source: "https://clouddocs.f5.com/api/irules/http_method.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "http_method" },
        ],
        ..CommandSpec::DEFAULT
    }
}
