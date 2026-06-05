//! `JSON::create` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::create",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Creates a new, empty JSON cache instance.",
            synopsis: &["JSON::create"],
            snippet: "Creates a new, empty JSON cache instance. It can then be filled with any JSON content and rendered. It will be deleted when no longer referenced by a Tcl variable.",
            source: "https://clouddocs.f5.com/api/irules/JSON__create.html",
            examples: "when JSON_REQUEST {\n    set cache [JSON::create]\n    set rootval [JSON::root $cache]\n    JSON::set $rootval string HelloWorld\n    set rendered [JSON::render $cache]\n}",
            return_value: "Returns the new JSON cache instance.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "JSON::create" },
        ],
        ..CommandSpec::DEFAULT
    }
}
