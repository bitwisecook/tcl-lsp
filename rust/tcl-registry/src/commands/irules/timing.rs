//! `timing` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "timing",
        traits: Traits::IRULES_TOP_LEVEL_ONLY,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enables or disables iRule timing statistics.",
            synopsis: &["timing TIMING"],
            snippet: "The timing command can be used to enable iRule timing statistics. This\nwill then collect timing information as specified each time the rule is\nevaluated. Statistics may be viewed with \"b rule show all\" or in the\nStatistics tab of the iRules Editor.\n\nNote: In 11.5.0, timing was enabled by default for all iRules in\nBZ375905. The performance impact is negligible. As a result, you no\nlonger need to use this command to view timing statistics.",
            source: "https://clouddocs.f5.com/api/irules/timing.html",
            examples: "when HTTP_REQUEST {\n    ...\n  }",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "timing TIMING" },
        ],
        ..CommandSpec::DEFAULT
    }
}
