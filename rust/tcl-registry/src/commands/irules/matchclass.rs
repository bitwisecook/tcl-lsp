//! `matchclass` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "matchclass",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Performs comparison against the contents of data group.",
            synopsis: &["matchclass CLASS_OR_VALUE KEYWORDS VALUE_OR_CLASS"],
            snippet: "Performs comparisons against the contents of data group. Typically used\nfor conditional logic control.\n\nNote: matchclass has been deprecated in v10 in favor of the new\nclass commands. The class command offers better functionality and\nperformance than matchclass.\n\nNote that you should not use a $:: or :: prefix on the datagroup name\nwhen using the matchclass command (or in any datagroup reference on\n9.4.4 or later).\n\nIn v9.4.4 - 10, using $::datagroup_name will work but demote the\nvirtual server from running on all TMMs. For details, see the CMP\ncompatibility page.",
            source: "https://clouddocs.f5.com/api/irules/matchclass.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [matchclass [IP::remote_addr] equals aol] } {\n     pool aol_pool\n  } else {\n     pool all_pool\n }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "matchclass CLASS_OR_VALUE KEYWORDS VALUE_OR_CLASS" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::DataGroup,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        deprecated_replacement: Some("class"),
        ..CommandSpec::DEFAULT
    }
}
