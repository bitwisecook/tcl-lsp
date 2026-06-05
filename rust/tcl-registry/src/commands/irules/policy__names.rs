//! `POLICY::names` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "POLICY::names",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns details about the policy names for the virtual server the iRule is enabled on.",
            synopsis: &["POLICY::names (active | matched | unmatched)"],
            snippet: "iRule command which returns details about the policy names for the\nvirtual server the iRule is enabled on.",
            source: "https://clouddocs.f5.com/api/irules/policy__names.html",
            examples: "# Log the policy names for this virtual server\nwhen HTTP_REQUEST {\n        log local0. \"Enabled on this VS: \\[POLICY::names active\\]: [POLICY::names active]\"\n        log local0. \"Matched: \\[POLICY::names matched\\]: [POLICY::names matched]\"\n        log local0. \"Not matched: \\[POLICY::names unmatched\\]: [POLICY::names unmatched]\"\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "POLICY::names (active | matched | unmatched)" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::BigipConfig,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
