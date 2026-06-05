//! `JSON::array` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::array",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "A group of subcommands that operate on a JSON array.",
            synopsis: &["JSON::array ("],
            snippet: "A group of subcommands that operate on a JSON array (first parameter of each subcommand).",
            source: "https://clouddocs.f5.com/api/irules/JSON__array.html",
            examples: "when JSON_REQUEST {\n    set rootval [JSON::root]\n    set ary [JSON::get $rootval array]\n\n    set size [JSON::array size $ary]\n    set type_at_idx [JSON::array type $ary 2]\n    set myint [JSON::array get $ary 1 integer]\n    JSON::array set $ary 0 integer 500\n    JSON::array insert $ary 5 string John\n    JSON::array append $ary null\n    JSON::array remove $ary 7\n    set myvaluelist [JSON::array values $ary]\n}",
            return_value: "Return depends on subcommand. See syntax description for detail.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "JSON::array (" },
        ],
        ..CommandSpec::DEFAULT
    }
}
