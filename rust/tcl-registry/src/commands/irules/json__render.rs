//! `JSON::render` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::render",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns a string containing a textual rendering of the JSON cache content.",
            synopsis: &["JSON::render (JSON_CACHE)?"],
            snippet: "If a JSON cache handle is omitted, renders any JSON cache that preexists in the context in which this is executed. This is the normal case when the command is executed in a the JSON_REQUEST or JSON_RESPONSE event.\nIf a JSON cache handle is provided, renders that JSON cache. This is useful when a JSON profile is not being used.\nNOTE: Rendering consumes the data in the cache, so after a render, no further value retrieval/modification/rendering may be done on this JSON cache instance.",
            source: "https://clouddocs.f5.com/api/irules/JSON__render.html",
            examples: "when MR_INGRESS {\n    set cache [JSON::create]\n    set rootval [JSON::root $cache]\n    JSON::set $rootval string HelloWorld\n    set rendered [JSON::render $cache]\n}",
            return_value: "Returns the string containing the rendered JSON content.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "JSON::render (JSON_CACHE)?" },
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
