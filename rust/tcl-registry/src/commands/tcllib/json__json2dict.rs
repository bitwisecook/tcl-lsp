//! `json::json2dict` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "json::json2dict jsonText",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "json::json2dict",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
hover: Some(HoverSnippet {
    summary: "Convert a JSON string to a Tcl dict.",
    synopsis: &["json::json2dict jsonText"],
    snippet: "Parses a JSON-encoded string and returns a nested Tcl dictionary. JSON objects become dicts, arrays become lists.",
    source: "tcllib json package",
    examples: "set data [json::json2dict $jsonString]",
    return_value: "A Tcl dict representing the JSON structure.",
}),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
