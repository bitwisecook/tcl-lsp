//! `AVR::log` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AVR::log",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Log an event for stats.",
            synopsis: &["AVR::log"],
            snippet: "AVR::log ssl_orchestrator Log ssl_orchestrator stats to AVR",
            source: "https://clouddocs.f5.com/api/irules/AVR__log.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "AVR::log",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::LogIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
