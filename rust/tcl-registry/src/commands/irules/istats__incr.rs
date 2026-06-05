//! `ISTATS::incr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ISTATS::incr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Increments the specified key by the given value.",
            synopsis: &["ISTATS::incr KEY VALUE"],
            snippet: "Increments the specified key by the given value. The increment value must be non-negative for a counter.\n\nNote that text string iStats may not be incremented.",
            source: "https://clouddocs.f5.com/api/irules/ISTATS__incr.html",
            examples: "when HTTP_REQUEST {\n        if { [string tolower [HTTP::uri]] equals \"/12345\" } {\n                ISTATS::incr \"uri /12345 counter Requests\" 1\n                HTTP::uri \"/\"\n                HTTP::redirect \"http://www.mysite.com\"\n        } elseif { [string tolower [HTTP::uri]] equals \"/stats\" } {\n                  HTTP::respond 200 content \"<html><body>Requests for /12345: [ISTATS::get \"uri /12345 counter Requests\"]</body></html>\"\n        }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ISTATS::incr KEY VALUE" },
        ],
        ..CommandSpec::DEFAULT
    }
}
