//! `HTTPLOG::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTPLOG::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "F5 iRules command `HTTPLOG::enable`.",
            synopsis: &["HTTPLOG::enable"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/HTTPLOG__enable.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "HTTPLOG::enable",
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
