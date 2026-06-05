//! `http_cookie` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http_cookie",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the value of the specified HTTP cookie.",
            synopsis: &["http_cookie ANY_CHARS"],
            snippet: "Returns the value in the Cookie: header for the specified cookie\nname. This is a BIG-IP version 4.X variable, provided for\nbackward-compatibility. You can use the equivalent 9.X command\nHTTP::cookie instead",
            source: "https://clouddocs.f5.com/api/irules/http_cookie.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "http_cookie ANY_CHARS" },
        ],
        ..CommandSpec::DEFAULT
    }
}
