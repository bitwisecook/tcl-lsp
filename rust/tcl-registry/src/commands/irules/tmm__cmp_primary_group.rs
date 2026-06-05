//! `TMM::cmp_primary_group` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TMM::cmp_primary_group",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "F5 iRules command `TMM::cmp_primary_group`.",
            synopsis: &["TMM::cmp_primary_group"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/TMM__cmp_primary_group.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
