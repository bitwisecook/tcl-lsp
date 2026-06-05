//! `nodes` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "nodes",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Lists all nodes within a given pool.",
            synopsis: &["nodes (-list)? POOL_OBJ"],
            snippet: "This command behaves like active_nodes but lists all nodes in a pool,\nnot just nodes that are currently active.",
            source: "https://clouddocs.f5.com/api/irules/nodes.html",
            examples: "when HTTP_REQUEST {\n        set in_path [HTTP::path]\n        log local0. \"debug request: path $in_path\"\n        switch -glob $in_path {\n                \"/pool*\" {\n                        set pool [string map {\"/pool\" \"\"} $in_path]\n                        HTTP::respond 200 content \"[active_members $pool]:[nodes $pool]\"\n                }\n        }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "nodes (-list)? POOL_OBJ" },
        ],
        options: &[
            OptionSpec { name: "-list", takes_value: false, value_hint: "", detail: "Option -list.", dialects: None },
        ],
        ..CommandSpec::DEFAULT
    }
}
