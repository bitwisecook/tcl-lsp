//! `JSON::set` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::set",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets a JSON element (aka.",
            synopsis: &["JSON::set JSON_ELEMENT JSON_TYPE (JSON_VALUE)?"],
            snippet: "Sets the value (content) of a JSON element, replacing any existing value. The given value should be according to the given type, as described below:\n\nnull: Omit. JSON type null has no value.\nboolean: 0 (false) or 1 (true).\ninteger: A Tcl number representing an integer in the range -(2^63) through (2^63 - 1). Otherwise, use the literal type.\nliteral: A Tcl string not requiring JSON escape sequences.\nstring: A Tcl string without escape sequences (certain characters will be replaced by JSON escape sequences).\nobject: Omit. An empty object is created.\narray: Omit. An empty array is created.",
            source: "https://clouddocs.f5.com/api/irules/JSON__set.html",
            examples: "when JSON_REQUEST {\n    set rootval [JSON::root]\n    JSON::set $rootval string HelloWorld\n}",
            return_value: "Returns the JSON element whose value was set (same element as the first argument passed to the command).",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "JSON::set JSON_ELEMENT JSON_TYPE (JSON_VALUE)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::Unknown,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::None,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
