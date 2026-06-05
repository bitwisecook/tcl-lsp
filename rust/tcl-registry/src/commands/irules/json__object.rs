//! `JSON::object` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::object",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "A group of subcommands that operate on a JSON object.",
            synopsis: &["JSON::object ("],
            snippet: "A group of subcommands that operate on a JSON object (first parameter of each subcommand).",
            source: "https://clouddocs.f5.com/api/irules/JSON__object.html",
            examples: "when JSON_REQUEST {\n    set rootval [JSON::root]\n    set obj [JSON::get $rootval object]\n\n    set size [JSON::object size $obj]\n    set type_at_key [JSON::object type $obj somekey]\n    set myint [JSON::object get $obj intkey integer]\n    JSON::object set $obj intkey integer 500\n    JSON::object add $obj namekey string John\n    JSON::object remove $obj intkey\n    set mykeylist [JSON::object keys $obj]\n    set myvaluelist [JSON::object values $obj]\n}",
            return_value: "Return depends on subcommand. See syntax description for detail.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "JSON::object (" },
        ],
        ..CommandSpec::DEFAULT
    }
}
