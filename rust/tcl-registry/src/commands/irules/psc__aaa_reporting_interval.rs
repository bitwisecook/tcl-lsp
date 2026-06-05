//! `PSC::aaa_reporting_interval` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::aaa_reporting_interval",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get or set AAA reporting interval.",
            synopsis: &["PSC::aaa_reporting_interval (INTERVAL)?"],
            snippet: "The PSC::aaa_reporting_interval command gets the AAA reporting interval or sets the AAA reporting interval when the optional value is given.",
            source: "https://clouddocs.f5.com/api/irules/PSC__aaa_reporting_interval.html",
            examples: "",
            return_value: "Return AAA reporting interval when no argument is given.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "PSC::aaa_reporting_interval (INTERVAL)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
