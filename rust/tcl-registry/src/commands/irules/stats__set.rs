//! `STATS::set` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "STATS::set",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the value of a Statistics profile setting.",
            synopsis: &["STATS::set PROFILE_NAME FIELD_NAME (VALUE)?"],
            snippet: "Sets the value of the specified setting (field), in the specified\nStatistics profile, to the specified value. If you do not specify a\nvalue, the BIG-IP system sets the value to 0.",
            source: "https://clouddocs.f5.com/api/irules/STATS__set.html",
            examples: "# Workaround for CR117956 (see version specific notes below for details)\nwhen RULE_INIT {\n    set ::stats_rst 1\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "STATS::set PROFILE_NAME FIELD_NAME (VALUE)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
