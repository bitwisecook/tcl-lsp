//! `ISTATS::get` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ISTATS::get",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Retrieves the value associated with the given key.",
            synopsis: &["ISTATS::get KEY"],
            snippet: "Reads in the value associated with the given iStats key",
            source: "https://clouddocs.f5.com/api/irules/ISTATS__get.html",
            examples: "when HTTP_REQUEST {\n        if { [string tolower [HTTP::uri]] equals \"/12345\" } {\n                ISTATS::incr \"uri /12345 counter Requests\" 1\n                HTTP::uri \"/\"\n                HTTP::redirect \"http://www.mysite.com\"\n        } elseif { [string tolower [HTTP::uri]] equals \"/stats\" } {\n                  HTTP::respond 200 content \"<html><body>Requests for /12345: [ISTATS::get \"uri /12345 counter Requests\"]</body></html>\"\n        }\n}",
            return_value: "Returns the value associated with the given key.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ISTATS::get KEY" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::IStats,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
