//! `TMM::cmp_groups` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TMM::cmp_groups",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "F5 iRules command `TMM::cmp_groups`.",
            synopsis: &["TMM::cmp_groups"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/TMM__cmp_groups.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TMM::cmp_groups",
        }],
        ..CommandSpec::DEFAULT
    }
}
