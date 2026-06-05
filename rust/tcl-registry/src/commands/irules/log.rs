//! `log` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "log",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Write a message to BIG-IP logging facilities.",
            synopsis: &["log ?facility.level? message"],
            snippet: "Common form: `log local0. \"message\"`.",
            source: "https://clouddocs.f5.com/api/irules/log.html",
            examples: "",
            return_value: "",
        }),
        // GAP-D2: tainted data in a log message → log injection /
        // forging (IRULE3003). Mirrors `irules/log.py`.
        taint_log_sink: Some("IRULE3003"),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "log ?-noname? ?facility.level? message",
        }],
        options: &[OptionSpec {
            name: "-noname",
            takes_value: false,
            value_hint: "",
            detail: "Suppress iRule name prefix in log message.",
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
