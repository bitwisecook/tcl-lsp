//! `traffic_group` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "traffic_group",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the current traffic group.",
            synopsis: &["traffic_group"],
            snippet: "This command returns the current traffic group. Useful for\ntroubleshooting the partitioning of data in the session table.",
            source: "https://clouddocs.f5.com/api/irules/traffic_group.html",
            examples: "when RULE_INIT {\n  log local0. \"traffic_group name: [traffic_group]\"\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "traffic_group" },
        ],
        ..CommandSpec::DEFAULT
    }
}
