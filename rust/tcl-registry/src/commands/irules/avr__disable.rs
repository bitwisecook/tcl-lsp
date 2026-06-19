//! `AVR::disable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AVR::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disables the AVR plugin for the current connection.",
            synopsis: &["AVR::disable"],
            snippet: "Disables the AVR plugin for the current connection. AVR will remain\ndisabled on the current connection until it is closed or\nAVR::enable is called. This means that the connection will not be\ncounted by AVR and thus excluded from statistics gathering.",
            source: "https://clouddocs.f5.com/api/irules/AVR__disable.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "AVR::disable",
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
