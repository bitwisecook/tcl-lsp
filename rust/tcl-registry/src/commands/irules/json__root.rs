//! `JSON::root` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::root",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets the JSON element (aka.",
            synopsis: &["JSON::root (JSON_CACHE)?"],
            snippet: "If a JSON cache handle is supplied, returns the root element of that JSON cache instance. This is useful when a JSON profile is not being used.",
            source: "https://clouddocs.f5.com/api/irules/JSON__root.html",
            examples: "when JSON_REQUEST {\n    set rootval [JSON::root]\n    JSON::set $rootval string HelloWorld\n    set rendered [JSON::render $cache]\n}",
            return_value: "Returns the JSON element at the root of the JSON cache instance.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "JSON::root (JSON_CACHE)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::Unknown,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::None,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
