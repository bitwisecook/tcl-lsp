//! `JSON::parse` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::parse",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Parses JSON content into a JSON cache that can be manipulated using further JSON:: commands.",
            synopsis: &["JSON::parse (JSON_STRING (JSON_MAX_ENTRIES)? )?"],
            snippet: "If a string is omitted, returns any JSON cache that preexists in the context in which this is executed. This is the normal case when the command is executed in the JSON_REQUEST or JSON_RESPONSE event.\nIf a string is provided, it is assumed to contain JSON and is parsed into a new JSON cache. This will be deleted when it is no longer referenced by a Tcl variable. This is useful when a JSON profile is not being used.",
            source: "https://clouddocs.f5.com/api/irules/JSON__parse.html",
            examples: "when JSON_REQUEST {\n    JSON::render\n}",
            return_value: "Returns a JSON cache instance handle to use for retrieving and overwriting content, and rendering.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "JSON::parse (JSON_STRING (JSON_MAX_ENTRIES)? )?" },
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
